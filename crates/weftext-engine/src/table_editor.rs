//! Bounded, exact-source table topology. This is an author-source projection,
//! not another table document or an evaluator of arbitrary Typst expressions.
use super::*;

const MAX_CELLS: usize = 100_000;

struct BindingCertificate<'a> {
    source: &'a str,
    known: std::cell::RefCell<std::collections::HashMap<String, bool>>,
}
impl BindingCertificate<'_> {
    fn available(&self, name: &str) -> bool {
        if let Some(value) = self.known.borrow().get(name).copied() {
            return value;
        }
        let value = standard_binding_available(self.source, name);
        self.known.borrow_mut().insert(name.into(), value);
        value
    }
}

#[derive(Clone)]
struct Cell {
    info: TableCellInfo,
    raw: String,
    body_start: usize,
    body_end: usize,
    /// Raw native style arguments; topology operations never invent styles.
    styles: Vec<String>,
}

struct NativeTable {
    columns: usize,
    rows: usize,
    cells: Vec<Cell>,
    header_rows: usize,
    /// Exact named options, including their surrounding whitespace.
    options: Vec<(String, String)>,
    prefix: String,
    separator: String,
    suffix: String,
    gaps: Vec<String>,
    header_layout: Option<(String, Vec<String>, String)>,
    column_expr: Range<usize>,
}

fn fail(message: &str) -> EngineError {
    EngineError(format!("table-unavailable: {message}"))
}

fn span_range(parsed: &Source, node: &SyntaxNode) -> Result<Range<usize>, EngineError> {
    parsed
        .find(node.span())
        .map(|n| n.range())
        .ok_or_else(|| fail("来源不可证明"))
}

fn integer(expr: ast::Expr<'_>) -> Option<usize> {
    expr.to_untyped()
        .cast::<ast::Int>()
        .and_then(|i| usize::try_from(i.get()).ok())
}

fn field_call(call: ast::FuncCall<'_>, name: &str) -> bool {
    call.callee()
        .to_untyped()
        .cast::<ast::FieldAccess>()
        .is_some_and(|f| {
            f.target()
                .to_untyped()
                .cast::<ast::Ident>()
                .is_some_and(|i| i.get() == "table")
                && f.field().get() == name
        })
}

/// Scalar native presentation options can be kept verbatim. Expressions that
/// depend on coordinates, generated values or unknown bindings are not inverted.
fn static_option(node: &SyntaxNode, certificate: &BindingCertificate<'_>) -> bool {
    if matches!(
        node.kind(),
        SyntaxKind::FuncCall | SyntaxKind::Closure | SyntaxKind::Spread
    ) {
        return false;
    }
    if node.cast::<ast::Array>().is_some() {
        return false;
    }
    if let Some(i) = node.cast::<ast::Ident>() {
        return certificate.available(i.get().as_str())
            && matches!(
                i.get().as_str(),
                "auto"
                    | "none"
                    | "left"
                    | "right"
                    | "center"
                    | "top"
                    | "bottom"
                    | "horizon"
                    | "start"
                    | "end"
                    | "black"
                    | "white"
                    | "red"
                    | "blue"
                    | "green"
                    | "gray"
                    | "silver"
                    | "yellow"
                    | "aqua"
                    | "fuchsia"
                    | "maroon"
                    | "navy"
                    | "olive"
                    | "purple"
                    | "teal"
                    | "lime"
            );
    }
    node.children()
        .all(|child| static_option(child, certificate))
}

fn topology_rules_known(node: &SyntaxNode, certificate: &BindingCertificate<'_>) -> bool {
    if let Some(set) = node.cast::<ast::SetRule>() {
        let target = set.target().to_untyped();
        if target.cast::<ast::FieldAccess>().is_some_and(|f| {
            f.target().to_untyped().cast::<ast::Ident>().is_some_and(|i| i.get() == "table")
                && f.field().get() == "cell"
        }) && set.args().items().any(|arg| {
            !matches!(arg, ast::Arg::Named(n) if !matches!(n.name().get().as_str(), "x" | "y" | "rowspan" | "colspan") && static_option(n.expr().to_untyped(), certificate))
        }) {
            return false;
        }
        if target
            .cast::<ast::Ident>()
            .is_some_and(|i| i.get() == "table")
            && set.args().items().any(
                |arg| !matches!(arg, ast::Arg::Named(n) if static_option(n.expr().to_untyped(), certificate)),
            )
        {
            return false;
        }
    }
    node.children()
        .all(|child| topology_rules_known(child, certificate))
}

fn parse_cell(
    expr: ast::Expr<'_>,
    header: bool,
    source: &str,
    parsed: &Source,
    certificate: &BindingCertificate<'_>,
) -> Result<Cell, EngineError> {
    let full = span_range(parsed, expr.to_untyped())?;
    let mut rowspan = 1;
    let mut colspan = 1;
    let mut styles = vec![];
    let block = if let Some(block) = expr.to_untyped().cast::<ast::ContentBlock>() {
        block
    } else {
        let call = expr
            .to_untyped()
            .cast::<ast::FuncCall>()
            .ok_or_else(|| fail("动态单元格保源"))?;
        if !field_call(call, "cell") || uncertain_table_trivia(expr.to_untyped()) {
            return Err(fail("仅支持显式 table.cell，包装注释归属不明时保源"));
        }
        let mut body = None;
        let mut names = std::collections::HashSet::new();
        for arg in call.args().items() {
            match arg {
                ast::Arg::Pos(e) if body.is_none() => {
                    body = Some(
                        e.to_untyped()
                            .cast::<ast::ContentBlock>()
                            .ok_or_else(|| fail("cell 的位置参数须为唯一显式内容块"))?,
                    )
                }
                ast::Arg::Named(n) => {
                    let name = n.name().get();
                    if !names.insert(name.to_string()) {
                        return Err(fail("重复 cell 参数保源"));
                    }
                    match name.as_str() {
                        "body" if body.is_none() => {
                            body = n.expr().to_untyped().cast::<ast::ContentBlock>()
                        }
                        "rowspan" => {
                            rowspan = integer(n.expr())
                                .filter(|n| *n > 0)
                                .ok_or_else(|| fail("动态 rowspan 保源"))?
                        }
                        "colspan" => {
                            colspan = integer(n.expr())
                                .filter(|n| *n > 0)
                                .ok_or_else(|| fail("动态 colspan 保源"))?
                        }
                        "inset" | "align" | "fill" | "stroke" | "breakable"
                            if static_option(n.expr().to_untyped(), certificate) =>
                        {
                            styles.push(source[span_range(parsed, n.to_untyped())?].to_string());
                        }
                        _ => return Err(fail("坐标、动态样式或未知 cell 参数保源")),
                    }
                }
                _ => return Err(fail("生成或重复 cell body 保源")),
            }
        }
        body.ok_or_else(|| fail("cell 需要唯一显式内容块"))?
    };
    let body = span_range(parsed, block.body().to_untyped())?;
    if full.start > body.start || body.end > full.end {
        return Err(fail("内容来源不在单元格内"));
    }
    Ok(Cell {
        info: TableCellInfo {
            row: 0,
            column: 0,
            rowspan,
            colspan,
            body: source_range(source, body.clone()),
            range: source_range(source, full.clone()),
            header,
        },
        raw: source[full.clone()].into(),
        body_start: body.start - full.start,
        body_end: body.end - full.start,
        styles,
    })
}

fn parse(
    node: &LinkedNode<'_>,
    call: ast::FuncCall<'_>,
    source: &str,
    parsed: &Source,
) -> Result<NativeTable, EngineError> {
    let certificate = BindingCertificate {
        source,
        known: std::cell::RefCell::new(std::collections::HashMap::new()),
    };
    if uncertain_table_trivia(node.get()) || !topology_rules_known(parsed.root(), &certificate) {
        return Err(fail("表外或包装注释、动态 table/cell 规则不能可靠重定位"));
    }
    let mut columns = None;
    let mut column_expr = None;
    let mut cells = vec![];
    let mut options = vec![];
    let mut header_seen = false;
    let mut data_seen = false;
    let mut first_cell = None;
    let mut last_cell = None;
    let mut argument_ranges: Vec<Range<usize>> = vec![];
    let mut header_range = None;
    for arg in call.args().items() {
        match arg {
            ast::Arg::Named(n) => {
                if first_cell.is_some() {
                    return Err(fail("单元格之后的命名参数暂不可安全重排"));
                }
                let name = n.name().get().to_string();
                if options.iter().any(|(key, _)| key == &name) {
                    return Err(fail("重复表格参数保源"));
                }
                if name == "columns" {
                    columns = integer(n.expr()).filter(|n| *n > 0);
                    if columns.is_none() {
                        return Err(fail("列数需要显式正整数"));
                    }
                    let expr = span_range(parsed, n.expr().to_untyped())?;
                    column_expr =
                        Some(expr.start - node.range().start..expr.end - node.range().start);
                } else if !matches!(
                    name.as_str(),
                    "fill"
                        | "stroke"
                        | "inset"
                        | "align"
                        | "gutter"
                        | "row-gutter"
                        | "column-gutter"
                ) || !static_option(n.expr().to_untyped(), &certificate)
                {
                    return Err(fail("行列格式或动态参数不能可靠重排"));
                }
                options.push((
                    name,
                    source[span_range(parsed, n.to_untyped())?].to_string(),
                ));
            }
            ast::Arg::Pos(e) => {
                let arg_range = span_range(parsed, e.to_untyped())?;
                first_cell.get_or_insert(arg_range.start);
                last_cell = Some(arg_range.end);
                argument_ranges.push(arg_range.clone());
                if let Some(header) = e
                    .to_untyped()
                    .cast::<ast::FuncCall>()
                    .filter(|c| field_call(*c, "header"))
                {
                    if header_seen || data_seen {
                        return Err(fail("只支持开头唯一的显式行表头"));
                    }
                    header_seen = true;
                    header_range = Some(arg_range);
                    let count = cells.len();
                    let mut header_names = std::collections::HashSet::new();
                    for arg in header.args().items() {
                        match arg {
                            ast::Arg::Pos(e) => {
                                cells.push(parse_cell(e, true, source, parsed, &certificate)?)
                            }
                            ast::Arg::Named(n) if cells.len() == count => {
                                let name = n.name().get();
                                if !header_names.insert(name.to_string())
                                    || !match name.as_str() {
                                        "repeat" => {
                                            n.expr().to_untyped().cast::<ast::Bool>().is_some()
                                        }
                                        "level" => integer(n.expr()) == Some(1),
                                        _ => false,
                                    }
                                {
                                    return Err(fail("动态或多级表头保源"));
                                }
                            }
                            _ => return Err(fail("带参数的多级/动态表头保源")),
                        }
                    }
                    if cells.len() == count {
                        return Err(fail("空表头无法证明完整逻辑行"));
                    }
                } else {
                    data_seen = true;
                    cells.push(parse_cell(e, false, source, parsed, &certificate)?);
                }
            }
            _ => return Err(fail("生成 cells 或 spread 保源")),
        }
    }
    let columns = columns.ok_or_else(|| fail("缺少显式列数"))?;
    if cells.is_empty() || columns > 1000 {
        return Err(fail("需要有界显式单元格"));
    }
    let rows = place(&mut cells, columns)?;
    let header_rows = cells
        .iter()
        .filter(|c| c.info.header)
        .map(|c| c.info.row + c.info.rowspan)
        .max()
        .unwrap_or(0);
    if cells.iter().any(|c| {
        c.info.header != (c.info.row < header_rows)
            || c.info.row < header_rows && c.info.row + c.info.rowspan > header_rows
    }) {
        return Err(fail("表头必须覆盖完整逻辑行"));
    }
    let first = first_cell.ok_or_else(|| fail("表格为空"))?;
    let last = last_cell.unwrap();
    let first_info = &cells[0].info.range;
    let separator = if cells.len() > 1 && !header_seen {
        let gap = &source[first_info.to..cells[1].info.range.from];
        if gap.contains(',') && gap.chars().all(|c| c == ',' || c.is_whitespace()) {
            gap.into()
        } else {
            ", ".into()
        }
    } else {
        ", ".into()
    };
    let gaps = argument_ranges
        .windows(2)
        .map(|pair| source[pair[0].end..pair[1].start].to_string())
        .collect();
    let header_layout = header_range.map(|range| {
        let header: Vec<_> = cells.iter().filter(|c| c.info.header).collect();
        let prefix = source[range.start..header[0].info.range.from].to_string();
        let suffix = source[header.last().unwrap().info.range.to..range.end].to_string();
        let gaps = header
            .windows(2)
            .map(|pair| source[pair[0].info.range.to..pair[1].info.range.from].to_string())
            .collect();
        (prefix, gaps, suffix)
    });
    Ok(NativeTable {
        columns,
        rows,
        cells,
        header_rows,
        options,
        prefix: source[node.range().start..first].into(),
        separator,
        suffix: source[last..node.range().end].into(),
        gaps,
        header_layout,
        column_expr: column_expr.unwrap(),
    })
}

fn place(cells: &mut [Cell], columns: usize) -> Result<usize, EngineError> {
    let mut slots = vec![];
    let mut cursor = 0;
    for cell in cells {
        while slots.get(cursor).copied().unwrap_or(false) {
            cursor += 1;
        }
        let row = cursor / columns;
        let column = cursor % columns;
        let height = row
            .checked_add(cell.info.rowspan)
            .ok_or_else(|| fail("表格超出预算"))?;
        let end = height
            .checked_mul(columns)
            .filter(|n| *n <= MAX_CELLS)
            .ok_or_else(|| fail("表格超出预算"))?;
        if column
            .checked_add(cell.info.colspan)
            .is_none_or(|n| n > columns)
        {
            return Err(fail("跨格越过行边界"));
        }
        slots.resize(slots.len().max(end), false);
        for y in row..height {
            for x in column..column + cell.info.colspan {
                let slot = y * columns + x;
                if slots[slot] {
                    return Err(fail("跨格重叠"));
                }
                slots[slot] = true;
            }
        }
        cell.info.row = row;
        cell.info.column = column;
    }
    if slots.is_empty() || slots.iter().any(|filled| !filled) {
        return Err(fail("不完整矩形保源"));
    }
    Ok(slots.len() / columns)
}

pub(super) fn inspect(
    node: &LinkedNode<'_>,
    call: ast::FuncCall<'_>,
    source: &str,
    parsed: &Source,
    fallback_columns: usize,
    cells: Vec<SourceRange>,
    grid_writable: bool,
) -> TableInfo {
    match parse(node, call, source, parsed) {
        Ok(table) => TableInfo {
            columns: table.columns,
            rows: table.rows,
            cells,
            grid_writable,
            logical_cells: table.cells.into_iter().map(|c| c.info).collect(),
            topology_writable: true,
            unavailable_reason: None,
            header_rows: table.header_rows,
            header_columns: 0,
        },
        Err(error) => TableInfo {
            columns: fallback_columns,
            rows: cells.len() / fallback_columns,
            cells,
            grid_writable,
            logical_cells: vec![],
            topology_writable: false,
            unavailable_reason: Some(error.0),
            header_rows: 0,
            header_columns: 0,
        },
    }
}

fn find_call<'a>(node: &LinkedNode<'a>, range: &SourceRange) -> Option<LinkedNode<'a>> {
    if node.range().start == range.from
        && node.range().end == range.to
        && node.get().cast::<ast::FuncCall>().is_some()
    {
        return Some(node.clone());
    }
    node.children().find_map(|c| find_call(&c, range))
}

fn selected(
    table: &NativeTable,
    top: usize,
    left: usize,
    bottom: usize,
    right: usize,
) -> Result<Vec<usize>, EngineError> {
    if top > bottom || left > right || bottom >= table.rows || right >= table.columns {
        return Err(fail("选区超出当前表格"));
    }
    let mut indices = vec![];
    for (index, cell) in table.cells.iter().enumerate() {
        let c = &cell.info;
        if c.row <= bottom
            && c.row + c.rowspan > top
            && c.column <= right
            && c.column + c.colspan > left
        {
            if c.row < top
                || c.column < left
                || c.row + c.rowspan > bottom + 1
                || c.column + c.colspan > right + 1
            {
                return Err(fail("选区必须完整覆盖合并格"));
            }
            indices.push(index);
        }
    }
    Ok(indices)
}

fn rewrite_cell(cell: &mut Cell, rowspan: usize, colspan: usize, body: Option<String>) {
    if rowspan == cell.info.rowspan && colspan == cell.info.colspan && body.is_none() {
        return;
    }
    let text = body.unwrap_or_else(|| cell.info.body.raw.clone());
    let mut args = cell.styles.clone();
    if rowspan > 1 {
        args.push(format!("rowspan: {rowspan}"));
    }
    if colspan > 1 {
        args.push(format!("colspan: {colspan}"));
    }
    cell.raw = if args.is_empty() {
        format!("[{text}]")
    } else {
        format!("table.cell({})[{text}]", args.join(", "))
    };
    cell.body_start = cell.raw.len() - text.len() - 1;
    cell.body_end = cell.raw.len() - 1;
    cell.info.body.raw = text;
    cell.info.rowspan = rowspan;
    cell.info.colspan = colspan;
}

fn blank(row: usize, column: usize, header: bool) -> Cell {
    let empty = SourceRange {
        from: 0,
        to: 0,
        utf16_from: 0,
        utf16_to: 0,
        raw: String::new(),
        writable: true,
    };
    Cell {
        info: TableCellInfo {
            row,
            column,
            rowspan: 1,
            colspan: 1,
            body: empty.clone(),
            range: empty,
            header,
        },
        raw: "[]".into(),
        body_start: 1,
        body_end: 1,
        styles: vec![],
    }
}

fn complete(table: &mut NativeTable) -> Result<(), EngineError> {
    check_grid(
        table.columns,
        table
            .rows
            .checked_mul(table.columns)
            .ok_or_else(|| fail("表格超出预算"))?,
    )?;
    table.cells.sort_by_key(|c| (c.info.row, c.info.column));
    let mut occupied = vec![false; table.rows * table.columns];
    for cell in &mut table.cells {
        let c = &cell.info;
        if c.rowspan == 0
            || c.colspan == 0
            || c.row + c.rowspan > table.rows
            || c.column + c.colspan > table.columns
        {
            return Err(fail("跨格超出目标表格"));
        }
        for y in c.row..c.row + c.rowspan {
            for x in c.column..c.column + c.colspan {
                let index = y * table.columns + x;
                if occupied[index] {
                    return Err(fail("目标表格跨格重叠"));
                }
                occupied[index] = true;
            }
        }
        cell.info.header = cell.info.row < table.header_rows;
        if cell.info.header && cell.info.row + cell.info.rowspan > table.header_rows {
            return Err(fail("合并格跨越表头边界"));
        }
    }
    if occupied.iter().any(|c| !c) {
        return Err(fail("目标表格有未覆盖槽"));
    }
    Ok(())
}

fn serialize(table: &NativeTable) -> String {
    // When column count is stable, keep every original named-option byte.
    let mut prefix = table.prefix.clone();
    prefix.replace_range(table.column_expr.clone(), &table.columns.to_string());
    let join = |items: &[String], gaps: &[String]| {
        let mut out = String::new();
        for (index, item) in items.iter().enumerate() {
            if index > 0 {
                out.push_str(gaps.get(index - 1).unwrap_or(&table.separator));
            }
            out.push_str(item);
        }
        out
    };
    let header: Vec<_> = table
        .cells
        .iter()
        .filter(|c| c.info.header)
        .map(|c| c.raw.clone())
        .collect();
    let mut items = vec![];
    if !header.is_empty() {
        items.push(if let Some((prefix, gaps, suffix)) = &table.header_layout {
            format!("{prefix}{}{suffix}", join(&header, gaps))
        } else {
            format!("table.header({})", header.join(&table.separator))
        });
    }
    items.extend(
        table
            .cells
            .iter()
            .filter(|c| !c.info.header)
            .map(|c| c.raw.clone()),
    );
    format!(
        "{prefix}{}{suffix}",
        join(&items, &table.gaps),
        suffix = table.suffix
    )
}

fn axis_insert(table: &mut NativeTable, index: usize, column: bool) -> Result<(), EngineError> {
    let length = if column { table.columns } else { table.rows };
    if index > length {
        return Err(fail("插入位置超出范围"));
    }
    let rows = table
        .rows
        .checked_add(usize::from(!column))
        .ok_or_else(|| fail("表格超出预算"))?;
    let columns = table
        .columns
        .checked_add(usize::from(column))
        .ok_or_else(|| fail("表格超出预算"))?;
    check_grid(
        columns,
        rows.checked_mul(columns)
            .ok_or_else(|| fail("表格超出预算"))?,
    )?;
    for cell in &table.cells {
        let start = if column {
            cell.info.column
        } else {
            cell.info.row
        };
        let span = if column {
            cell.info.colspan
        } else {
            cell.info.rowspan
        };
        if start < index && index < start + span {
            return Err(fail("插入线穿过合并格，请先拆分"));
        }
    }
    for cell in &mut table.cells {
        let coordinate = if column {
            &mut cell.info.column
        } else {
            &mut cell.info.row
        };
        if *coordinate >= index {
            *coordinate += 1;
        }
    }
    if column {
        table.columns += 1;
        for row in 0..table.rows {
            table.cells.push(blank(row, index, row < table.header_rows));
        }
    } else {
        table.rows += 1;
        if index < table.header_rows {
            table.header_rows += 1;
        }
        for column in 0..table.columns {
            table
                .cells
                .push(blank(index, column, index < table.header_rows));
        }
    }
    Ok(())
}

fn axis_remove(table: &mut NativeTable, index: usize, column: bool) -> Result<(), EngineError> {
    let length = if column { table.columns } else { table.rows };
    if index >= length || length == 1 {
        return Err(fail("不能删除不存在或最后一行/列"));
    }
    for cell in &table.cells {
        let start = if column {
            cell.info.column
        } else {
            cell.info.row
        };
        let span = if column {
            cell.info.colspan
        } else {
            cell.info.rowspan
        };
        if start <= index && index < start + span && span != 1 {
            return Err(fail("删除范围穿过合并格，请先拆分"));
        }
    }
    table.cells.retain(|cell| {
        (if column {
            cell.info.column
        } else {
            cell.info.row
        }) != index
    });
    for cell in &mut table.cells {
        let coordinate = if column {
            &mut cell.info.column
        } else {
            &mut cell.info.row
        };
        if *coordinate > index {
            *coordinate -= 1;
        }
    }
    if column {
        table.columns -= 1;
    } else {
        table.rows -= 1;
        if index < table.header_rows {
            table.header_rows -= 1;
        }
    }
    Ok(())
}

fn axis_move(
    table: &mut NativeTable,
    from: usize,
    to: usize,
    column: bool,
) -> Result<(), EngineError> {
    let length = if column { table.columns } else { table.rows };
    if from >= length || to >= length {
        return Err(fail("移动位置超出范围"));
    }
    let map = |index: usize| {
        if index == from {
            to
        } else if from < to && index > from && index <= to {
            index - 1
        } else if to < from && index >= to && index < from {
            index + 1
        } else {
            index
        }
    };
    for cell in &mut table.cells {
        let start = if column {
            cell.info.column
        } else {
            cell.info.row
        };
        let span = if column {
            cell.info.colspan
        } else {
            cell.info.rowspan
        };
        let positions: Vec<_> = (start..start + span).map(map).collect();
        let minimum = *positions.iter().min().unwrap();
        let maximum = *positions.iter().max().unwrap();
        if maximum - minimum + 1 != span {
            return Err(fail("移动会切断合并格，请先拆分"));
        }
        if !column
            && positions
                .iter()
                .any(|r| (*r < table.header_rows) != cell.info.header)
        {
            return Err(fail("移动不能静默改变表头归属"));
        }
        if column {
            cell.info.column = minimum;
        } else {
            cell.info.row = minimum;
        }
    }
    Ok(())
}

fn paste_cells(
    table: &mut NativeTable,
    row: usize,
    column: usize,
    height: usize,
    width: usize,
    cells: &[TablePasteCell],
    expand: bool,
) -> Result<(), EngineError> {
    if height == 0
        || width == 0
        || height.checked_mul(width).is_none_or(|n| n > MAX_CELLS)
        || row >= table.rows
        || column >= table.columns
    {
        return Err(fail("粘贴目标或尺寸无效"));
    }
    let bottom = row
        .checked_add(height)
        .ok_or_else(|| fail("粘贴超出预算"))?;
    let right = column
        .checked_add(width)
        .ok_or_else(|| fail("粘贴超出预算"))?;
    if bottom.checked_mul(right).is_none_or(|n| n > MAX_CELLS) {
        return Err(fail("粘贴超出预算"));
    }
    let final_rows = table.rows.max(bottom);
    let final_columns = table.columns.max(right);
    check_grid(
        final_columns,
        final_rows
            .checked_mul(final_columns)
            .ok_or_else(|| fail("粘贴超出预算"))?,
    )?;
    if !expand && (bottom > table.rows || right > table.columns) {
        return Err(fail("粘贴超出现有范围，需要明确选择扩展"));
    }
    let mut coverage = vec![false; height * width];
    let mut replacements = vec![];
    let mut bytes = 0usize;
    for cell in cells {
        bytes = bytes
            .checked_add(cell.text.len())
            .filter(|n| *n <= 1_048_576)
            .ok_or_else(|| fail("剪贴板文字超出预算"))?;
        if cell.rowspan == 0
            || cell.colspan == 0
            || cell
                .row
                .checked_add(cell.rowspan)
                .is_none_or(|n| n > height)
            || cell
                .column
                .checked_add(cell.colspan)
                .is_none_or(|n| n > width)
        {
            return Err(fail("剪贴板跨格超出矩形"));
        }
        if cell.header != (row + cell.row < table.header_rows)
            || cell.header && row + cell.row + cell.rowspan > table.header_rows
        {
            return Err(fail("剪贴板表头与目标不兼容，需明确转换"));
        }
        for y in cell.row..cell.row + cell.rowspan {
            for x in cell.column..cell.column + cell.colspan {
                if coverage[y * width + x] {
                    return Err(fail("剪贴板跨格重叠"));
                }
                coverage[y * width + x] = true;
            }
        }
        let mut new = blank(row + cell.row, column + cell.column, cell.header);
        rewrite_cell(
            &mut new,
            cell.rowspan,
            cell.colspan,
            Some(encode_cell_literal(&cell.text)?),
        );
        replacements.push(new);
    }
    if coverage.iter().any(|c| !c) {
        return Err(fail("剪贴板未完整覆盖矩形"));
    }
    // Expansion only adds bottom/right slots. Build them once after checking
    // final dimensions, rather than repeatedly scanning the growing table.
    let (old_rows, old_columns) = (table.rows, table.columns);
    table.rows = final_rows;
    table.columns = final_columns;
    for y in 0..final_rows {
        for x in 0..final_columns {
            if y >= old_rows || x >= old_columns {
                table.cells.push(blank(y, x, y < table.header_rows));
            }
        }
    }
    let indices = selected(table, row, column, bottom - 1, right - 1)?;
    let index_set: std::collections::HashSet<_> = indices.iter().copied().collect();
    let original_by_position: std::collections::HashMap<_, _> = indices
        .iter()
        .map(|index| {
            let c = &table.cells[*index];
            (
                (c.info.row, c.info.column, c.info.rowspan, c.info.colspan),
                c,
            )
        })
        .collect();
    let mut original_slots = vec![None; table.rows * table.columns];
    for index in &indices {
        let c = &table.cells[*index].info;
        for y in c.row..c.row + c.rowspan {
            for x in c.column..c.column + c.colspan {
                original_slots[y * table.columns + x] = Some(*index);
            }
        }
    }
    // Keep native style and wrapper bytes when replacing a compatible cell.
    for new in &mut replacements {
        if let Some(old) = original_by_position.get(&(
            new.info.row,
            new.info.column,
            new.info.rowspan,
            new.info.colspan,
        )) {
            let body = new.info.body.raw.clone();
            *new = (*old).clone();
            new.raw.replace_range(new.body_start..new.body_end, &body);
            new.body_end = new.body_start + body.len();
            new.info.body.raw = body;
        } else {
            let mut origins = std::collections::HashSet::new();
            for y in new.info.row..new.info.row + new.info.rowspan {
                for x in new.info.column..new.info.column + new.info.colspan {
                    if let Some(index) = original_slots[y * table.columns + x] {
                        origins.insert(index);
                    }
                }
            }
            if let Some(first) = origins.iter().next().map(|index| &table.cells[*index]) {
                if origins
                    .iter()
                    .any(|index| table.cells[*index].styles != first.styles)
                {
                    return Err(fail("粘贴拓扑会合并不同原生格式，请先明确统一或拆分目标"));
                }
                new.styles = first.styles.clone();
                let (rowspan, colspan, body) = (
                    new.info.rowspan,
                    new.info.colspan,
                    new.info.body.raw.clone(),
                );
                rewrite_cell(new, rowspan, colspan, Some(body));
            }
        }
    }
    table.cells = table
        .cells
        .iter()
        .enumerate()
        .filter(|(i, _)| !index_set.contains(i))
        .map(|(_, c)| c.clone())
        .chain(replacements)
        .collect();
    Ok(())
}

pub(super) fn encode_clipboard(
    rows: usize,
    columns: usize,
    cells: &[TablePasteCell],
) -> Result<String, EngineError> {
    check_grid(
        columns,
        rows.checked_mul(columns)
            .ok_or_else(|| fail("表格超出预算"))?,
    )?;
    let header_rows = cells
        .iter()
        .filter(|cell| cell.header)
        .map(|cell| cell.row.saturating_add(cell.rowspan))
        .max()
        .unwrap_or(0);
    if header_rows > rows {
        return Err(fail("剪贴板表头超出行数"));
    }
    let mut table = NativeTable {
        rows,
        columns,
        header_rows,
        cells: (0..rows)
            .flat_map(|row| (0..columns).map(move |column| blank(row, column, row < header_rows)))
            .collect(),
        options: vec![("columns".into(), format!("columns: {columns}"))],
        prefix: format!("table(columns: {columns}, "),
        separator: ", ".into(),
        suffix: ")".into(),
        gaps: vec![],
        header_layout: None,
        column_expr: "table(columns: ".len().."table(columns: ".len() + columns.to_string().len(),
    };
    paste_cells(&mut table, 0, 0, rows, columns, cells, false)?;
    complete(&mut table)?;
    let desired: Vec<_> = table
        .cells
        .iter()
        .map(|c| (c.info.row, c.info.column))
        .collect();
    let mut verification = table.cells.clone();
    if place(&mut verification, columns)? != rows
        || verification
            .iter()
            .map(|c| (c.info.row, c.info.column))
            .collect::<Vec<_>>()
            != desired
    {
        return Err(fail("剪贴板拓扑无法由原生顺序表示"));
    }
    Ok(serialize(&table))
}

pub(super) fn edit(
    source: &str,
    structure: &StructureInfo,
    action: &TableAction,
    patches: &mut Vec<SourcePatch>,
) -> Result<(), EngineError> {
    let info = structure
        .table
        .as_ref()
        .ok_or_else(|| fail("不是当前表格"))?;
    if !info.topology_writable {
        return Err(fail(
            info.unavailable_reason.as_deref().unwrap_or("来源无法证明"),
        ));
    }
    let parsed = Source::detached(source);
    let node = find_call(&LinkedNode::new(parsed.root()), &structure.range)
        .ok_or_else(|| fail("表格来源已变化"))?;
    let call = node.get().cast::<ast::FuncCall>().unwrap();
    let mut table = parse(&node, call, source, &parsed)?;
    let original_cells = table.cells.clone();
    let original_dimensions = (table.rows, table.columns, table.header_rows);
    match action {
        TableAction::InsertRow { index } => axis_insert(&mut table, *index, false)?,
        TableAction::RemoveRow { index } => axis_remove(&mut table, *index, false)?,
        TableAction::InsertColumn { index } => axis_insert(&mut table, *index, true)?,
        TableAction::RemoveColumn { index } => axis_remove(&mut table, *index, true)?,
        TableAction::MoveRow { from, to } => axis_move(&mut table, *from, *to, false)?,
        TableAction::MoveColumn { from, to } => axis_move(&mut table, *from, *to, true)?,
        TableAction::ClearRange {
            top,
            left,
            bottom,
            right,
        } => {
            for index in selected(&table, *top, *left, *bottom, *right)? {
                let c = &table.cells[index].info.body;
                push_patch(patches, source, c.from..c.to, String::new());
            }
            return Ok(());
        }
        TableAction::Paste { row, column, text }
        | TableAction::PasteRange {
            row, column, text, ..
        } => {
            if *row >= table.rows || *column >= table.columns {
                return Err(fail("粘贴目标超出当前表格"));
            }
            let matrix = super::table_clipboard::parse_tsv(text)?;
            let width = matrix[0].len();
            let cells: Vec<_> = matrix
                .iter()
                .enumerate()
                .flat_map(|(y, line)| {
                    line.iter()
                        .enumerate()
                        .map(move |(x, text)| TablePasteCell {
                            row: y,
                            column: x,
                            rowspan: 1,
                            colspan: 1,
                            text: text.clone(),
                            header: row + y < info.header_rows,
                        })
                })
                .collect();
            paste_cells(
                &mut table,
                *row,
                *column,
                matrix.len(),
                width,
                &cells,
                matches!(action, TableAction::PasteRange { expand: true, .. }),
            )?;
        }
        TableAction::PasteCells {
            row,
            column,
            rows,
            columns,
            cells,
            expand,
        } => paste_cells(&mut table, *row, *column, *rows, *columns, cells, *expand)?,
        TableAction::Merge {
            top,
            left,
            bottom,
            right,
        } => {
            let indices = selected(&table, *top, *left, *bottom, *right)?;
            if indices.len() == 1 {
                return Ok(());
            }
            if *top < table.header_rows && *bottom >= table.header_rows {
                return Err(fail("不能跨越表头合并"));
            }
            let first = *indices.first().ok_or_else(|| fail("空选区"))?;
            let mut merged = table.cells[first].clone();
            if indices
                .iter()
                .any(|i| table.cells[*i].styles != merged.styles)
            {
                return Err(fail("各格格式不同，不能静默丢弃样式"));
            }
            let text = indices
                .iter()
                // Code can carry cell-local set/let/import scopes. Plain text
                // remains one editable run rather than acquiring code wrappers.
                .map(|i| {
                    let raw = &table.cells[*i].info.body.raw;
                    fn has_code(node: &SyntaxNode) -> bool {
                        node.kind() == SyntaxKind::Hash || node.children().any(has_code)
                    }
                    if has_code(Source::detached(raw).root()) {
                        format!("#[{raw}]")
                    } else {
                        raw.clone()
                    }
                })
                .collect::<Vec<_>>()
                .join("\\\n\\\n");
            rewrite_cell(&mut merged, bottom - top + 1, right - left + 1, Some(text));
            let index_set: std::collections::HashSet<_> = indices.into_iter().collect();
            table.cells = table
                .cells
                .iter()
                .enumerate()
                .filter(|(i, _)| !index_set.contains(i))
                .map(|(_, c)| c.clone())
                .chain([merged])
                .collect();
        }
        TableAction::Split { row, column } => {
            let index = table
                .cells
                .iter()
                .position(|c| c.info.row == *row && c.info.column == *column)
                .ok_or_else(|| fail("拆分需要合并格的逻辑起点"))?;
            let original = table.cells.remove(index);
            let mut first = original.clone();
            rewrite_cell(&mut first, 1, 1, None);
            table.cells.push(first);
            for y in original.info.row..original.info.row + original.info.rowspan {
                for x in original.info.column..original.info.column + original.info.colspan {
                    if y != *row || x != *column {
                        let mut new = blank(y, x, original.info.header);
                        new.styles = original.styles.clone();
                        rewrite_cell(&mut new, 1, 1, Some(String::new()));
                        table.cells.push(new);
                    }
                }
            }
        }
        TableAction::SetHeader { rows, columns } => {
            if *columns != 0 {
                return Err(fail("当前 Typst 原生 table.header 只支持整行表头"));
            }
            if *rows > table.rows {
                return Err(fail("表头超出行数"));
            }
            table.header_rows = *rows;
        }
        TableAction::Transpose => {
            if table.header_rows > 0 {
                return Err(fail("行表头无法无损变为原生列表头；请先明确取消表头再转置"));
            }
            if table.options.iter().any(|(name, _)| {
                matches!(
                    name.as_str(),
                    "row-gutter" | "column-gutter" | "align" | "inset" | "stroke"
                )
            }) || table.cells.iter().any(|c| !c.styles.is_empty())
            {
                return Err(fail("方向样式无法无损转置，请编辑表格源码"));
            }
            std::mem::swap(&mut table.columns, &mut table.rows);
            for cell in &mut table.cells {
                std::mem::swap(&mut cell.info.row, &mut cell.info.column);
                let rowspan = cell.info.colspan;
                let colspan = cell.info.rowspan;
                rewrite_cell(cell, rowspan, colspan, None);
            }
        }
    }
    complete(&mut table)?;
    if original_dimensions == (table.rows, table.columns, table.header_rows)
        && original_cells.len() == table.cells.len()
        && original_cells.iter().zip(&table.cells).all(|(old, new)| {
            (
                old.info.row,
                old.info.column,
                old.info.rowspan,
                old.info.colspan,
            ) == (
                new.info.row,
                new.info.column,
                new.info.rowspan,
                new.info.colspan,
            ) && old.raw[..old.body_start] == new.raw[..new.body_start]
                && old.raw[old.body_end..] == new.raw[new.body_end..]
        })
    {
        for (old, new) in original_cells.iter().zip(&table.cells) {
            if old.info.body.raw != new.info.body.raw {
                push_patch(
                    patches,
                    source,
                    old.info.body.from..old.info.body.to,
                    new.info.body.raw.clone(),
                );
            }
        }
        return Ok(());
    }
    // Row-major native placement must certify exactly the requested coordinates.
    let desired: Vec<_> = table
        .cells
        .iter()
        .map(|c| (c.info.row, c.info.column))
        .collect();
    let mut verification = table.cells.clone();
    if place(&mut verification, table.columns)? != table.rows
        || verification
            .iter()
            .map(|c| (c.info.row, c.info.column))
            .collect::<Vec<_>>()
            != desired
    {
        return Err(fail("目标拓扑不能由当前显式原生顺序表示"));
    }
    push_patch(
        patches,
        source,
        structure.range.from..structure.range.to,
        serialize(&table),
    );
    Ok(())
}
