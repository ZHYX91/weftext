import { uiDisplayParameter } from "./ui-locale";
import { message as uiMessage, displayMessage, setUiText, setUiAttr, setUiValidity, uiTextNode, uiTextValue, errorUiText, UiError, type UiText } from "./ui-locale";
/** Clipboard data is an untrusted proposal, never a native source address. */
export interface ClipboardCell {
    row: number;
    column: number;
    rowspan: number;
    colspan: number;
    text: string;
    header: boolean;
}
export interface ClipboardTable {
    rows: number;
    columns: number;
    cells: ClipboardCell[];
    losses: UiText[];
}
const LIMIT = 4096;
export function parseDelimited(text: string, delimiter: "\t" | ","): string[][] {
    if (text.length > 1000000)
        throw new UiError(uiMessage("tableClipboard.textBudgetExceeded"));
    const rows: string[][] = [[]];
    let cell = "", quoted = false, closed = false, start = true;
    for (let at = 0; at < text.length; at++) {
        const character = text[at];
        if (quoted) {
            if (character === '"') {
                if (text[at + 1] === '"') {
                    cell += '"';
                    at++;
                }
                else {
                    quoted = false;
                    closed = true;
                }
            }
            else
                cell += character;
        }
        else if (character === '"' && start) {
            quoted = true;
            start = false;
        }
        else if (character === delimiter) {
            rows.at(-1)!.push(cell);
            cell = "";
            start = true;
            closed = false;
        }
        else if (character === "\r" || character === "\n") {
            if (character === "\r" && text[at + 1] === "\n")
                at++;
            rows.at(-1)!.push(cell);
            rows.push([]);
            cell = "";
            start = true;
            closed = false;
        }
        else {
            if (closed)
                throw new UiError(uiMessage("tableClipboard.textAfterClosingQuote"));
            cell += character;
            start = false;
        }
        if (rows.length > LIMIT || rows.at(-1)!.length > LIMIT)
            throw new UiError(uiMessage("tableClipboard.cellBudgetExceeded"));
    }
    if (quoted)
        throw new UiError(uiMessage("tableClipboard.quoteUnclosed"));
    if (rows.length > 1 && !cell && !rows.at(-1)!.length && /[\r\n]$/.test(text))
        rows.pop();
    else
        rows.at(-1)!.push(cell);
    if (!rows[0]?.length || rows.some(row => row.length !== rows[0].length) || rows.length * rows[0].length > LIMIT)
        throw new UiError(uiMessage("tableClipboard.rectangularTableRequired"));
    return rows;
}
export function delimitedTable(text: string, delimiter: "\t" | ","): ClipboardTable {
    const grid = parseDelimited(text, delimiter);
    return { rows: grid.length, columns: grid[0].length, cells: grid.flatMap((row, r) => row.map((text, c) => ({ row: r, column: c, rowspan: 1, colspan: 1, text, header: false }))), losses: [] };
}
export function writeDelimited(rows: string[][], delimiter: "\t" | ","): string {
    return rows.map(row => row.map(text => !text && row.length === 1 || /["\r\n]/.test(text) || text.includes(delimiter) ? `"${text.replace(/"/g, '""')}"` : text).join(delimiter)).join("\r\n");
}
function htmlCellText(root: Node): string {
    let output = "", blockEnd = false;
    const boundary = () => {
        if (output && !output.endsWith("\n"))
            output += "\n";
    };
    const visit = (node: Node) => {
        if (node.nodeType === 3) {
            const text = node.textContent ?? "";
            if (text && blockEnd) {
                boundary();
                blockEnd = false;
            }
            output += text;
            return;
        }
        if (!(node instanceof Element))
            return;
        if (node.tagName === "BR") {
            if (blockEnd)
                boundary();
            output += "\n";
            blockEnd = false;
            return;
        }
        const block = /^(P|DIV|PRE)$/.test(node.tagName);
        if (block) {
            boundary();
            blockEnd = false;
        }
        Array.from(node.childNodes).forEach(visit);
        if (block)
            blockEnd = true;
    };
    Array.from(root.childNodes).forEach(visit);
    return output;
}
export function parseHTMLTable(html: string): ClipboardTable {
    if (html.length > 1000000)
        throw new UiError(uiMessage("tableClipboard.htmlBudgetExceeded"));
    // Template contents are inert, including resource elements outside the table.
    // Do not use a live detached Document: some engines fetch its img/iframe URLs.
    const template = document.createElement("template");
    template.innerHTML = html;
    const tables = template.content.querySelectorAll("table");
    if (tables.length !== 1)
        throw new UiError(uiMessage("tableClipboard.singleNonNestedTableRequired"));
    const table = tables[0];
    const outside = template.content.cloneNode(true) as DocumentFragment;
    outside.querySelector("table")!.remove();
    outside.querySelectorAll("style,meta,link").forEach(node => node.remove());
    if (outside.textContent?.trim() || outside.querySelector("img,svg,iframe,object,embed,video,audio,canvas,table"))
        throw new UiError(uiMessage("tableClipboard.contentOutsideTable"));
    if (table.querySelector("script,style,iframe,object,embed,img,svg,math,video,audio,canvas,input,button"))
        throw new UiError(uiMessage("tableClipboard.htmlNonTextObjects"));
    if (Array.from(table.querySelectorAll("*")).some(node => !/^(THEAD|TBODY|TFOOT|TR|TD|TH|BR|P|DIV|SPAN|A|B|STRONG|I|EM|U|S|STRIKE|CODE|PRE|SMALL|SUB|SUP|FONT|COL|COLGROUP)$/.test(node.tagName)))
        throw new UiError(uiMessage("tableClipboard.htmlUnsupportedStructure"));
    const cells: ClipboardCell[] = [], occupied = new Set<string>();
    let columns = 0;
    const rows = Array.from(table.rows);
    if (!rows.length || rows.length > LIMIT)
        throw new UiError(uiMessage("tableClipboard.htmlEmptyOrOversized"));
    rows.forEach((row, r) => {
        let c = 0;
        for (const cell of Array.from(row.cells)) {
            while (occupied.has(`${r}:${c}`))
                c++;
            const rowspan = Number(cell.getAttribute("rowspan") ?? 1), colspan = Number(cell.getAttribute("colspan") ?? 1);
            if (!Number.isSafeInteger(rowspan) || !Number.isSafeInteger(colspan) || rowspan < 1 || colspan < 1 || r + rowspan > rows.length || colspan > LIMIT)
                throw new UiError(uiMessage("tableClipboard.htmlSpanInvalid"));
            for (let rr = r; rr < r + rowspan; rr++)
                for (let cc = c; cc < c + colspan; cc++) {
                    const key = `${rr}:${cc}`;
                    if (occupied.has(key))
                        throw new UiError(uiMessage("tableClipboard.htmlSpansOverlap"));
                    occupied.add(key);
                    if (occupied.size > LIMIT)
                        throw new UiError(uiMessage("tableClipboard.htmlCellBudgetExceeded"));
                }
            cells.push({ row: r, column: c, rowspan, colspan, text: htmlCellText(cell), header: cell.tagName === "TH" });
            c += colspan;
            columns = Math.max(columns, c);
        }
    });
    if (rows.length * columns !== occupied.size)
        throw new UiError(uiMessage("tableClipboard.htmlMissingCells"));
    const losses = table.querySelector("a,b,strong,i,em,u,s,strike,code,small,sub,sup,font,[style],[class],tfoot") || template.content.querySelector("style,link") ? [uiMessage("tableClipboard.htmlFormattingLoss")] : [];
    return { rows: rows.length, columns, cells, losses };
}
const escapeHTML = (text: string) => text.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;");
export function writeHTMLTable(table: ClipboardTable): string {
    return "<table><tbody>" + Array.from({ length: table.rows }, (_, r) => "<tr>" + table.cells.filter(cell => cell.row === r).map(cell => {
        const tag = cell.header ? "th" : "td";
        return `<${tag} rowspan="${cell.rowspan}" colspan="${cell.colspan}">${escapeHTML(cell.text).replace(/\r\n|\r|\n/g, "<br>")}</${tag}>`;
    }).join("") + "</tr>").join("") + "</tbody></table>";
}
/** Flat formats put a merged cell's text at its origin and empty covered slots. */
export function flatClipboard(table: ClipboardTable): string[][] {
    const rows = Array.from({ length: table.rows }, () => Array<string>(table.columns).fill(""));
    for (const cell of table.cells)
        rows[cell.row][cell.column] = cell.text;
    return rows;
}
