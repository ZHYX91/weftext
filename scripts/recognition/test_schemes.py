"""Pure proposal tests; these are not OCR, HTTP, or product acceptance."""

from __future__ import annotations

import copy
import unittest

from schemes import INVOICE, OFFICIAL, SchemeInputError, propose

SETTINGS = {"language": "zh", "script": "Hans", "region": "CN"}


def block(ident, text, box=None, **kwargs):
    return {"id": ident, "text": text, "bbox": box, "sourceMethod": "text-layer", **kwargs}


def document(blocks, media_type="application/pdf"):
    return {
        "version": 1, "input": {"mediaType": media_type, "sha256": "0" * 64, "bytes": 123},
        "status": "complete", "pageCount": 1, "pages": [{"page": 1, "status": "complete", "width": 720, "height": 480,
            "regions": [{"id": "page-1", "status": "complete", "method": "text-layer", "blocks": blocks}]}],
    }


def invoice():
    return document([
        block("title", "电子发票（普通发票）", [180, 20, 450, 42]),
        block("number", "发票号码：00123456789012345678", [470, 50, 710, 65]),
        block("date", "开票日期：2026年10月18日", [470, 70, 710, 85]),
        block("buyer", "购买方信息", [10, 100, 26, 160]),
        block("seller", "销售方信息", [360, 100, 376, 160]),
        block("buyer-name", "名称：TEST购买企业", [40, 105, 250, 120]),
        block("buyer-tax", "统一社会信用代码/纳税人识别号：001234567890123456", [40, 130, 335, 145]),
        block("seller-name", "名称：TEST销售企业", [390, 105, 650, 120]),
        block("seller-tax", "统一社会信用代码/纳税人识别号：991234567890123456", [390, 130, 710, 145]),
        block("amount", "金额合计：1,000.00", [440, 310, 540, 325]),
        block("tax", "税额合计：130.00", [560, 310, 710, 325]),
        block("total", "价税合计（小写）：1,130.00", [440, 340, 710, 355]),
        block("currency", "币种：人民币", [40, 340, 220, 355]),
        block("test", "TEST ONLY — 合成测试，非有效发票", [20, 430, 700, 460]),
    ])


class InvoiceSchemeTests(unittest.TestCase):
    def test_field_value_kinds_keep_leading_zero_identifiers_as_text(self):
        fields = propose(INVOICE, invoice(), SETTINGS)["fields"]
        self.assertEqual(fields["invoiceNumber"]["valueKind"], "text")
        self.assertEqual(fields["buyerTaxId"]["valueKind"], "text")
        self.assertEqual(fields["issueDate"]["valueKind"], "date")
        self.assertEqual(fields["amount"]["valueKind"], "decimal")
        self.assertEqual(fields["total"]["value"], "1130.00")

    def test_party_geometry_uses_original_page_units_not_fixed_pixel_band(self):
        source = invoice()
        source["input"]["mediaType"] = "image/png"
        page = source["pages"][0]
        page["width"], page["height"] = 3750, 2417
        positions = {"buyer": [160, 572, 560, 654], "seller": [1950, 572, 2247, 654],
            "buyer-name": [160, 715, 1000, 787], "seller-name": [1950, 715, 2700, 787],
            "buyer-tax": [160, 853, 1700, 921], "seller-tax": [1950, 853, 3600, 921]}
        for item in page["regions"][0]["blocks"]:
            if item["id"] in positions:
                item["bbox"] = positions[item["id"]]
        fields = propose(INVOICE, source, SETTINGS)["fields"]
        self.assertEqual(fields["buyerName"]["value"], "TEST购买企业")
        self.assertEqual(fields["sellerTaxId"]["value"], "991234567890123456")

    def test_explicit_total_label_does_not_guess_noisy_optional_qualifier(self):
        source = invoice()
        source["pages"][0]["regions"][0]["blocks"][-3]["text"] = "价 税 合计 (hE): ¥1,130.00"
        result = propose(INVOICE, source, SETTINGS)
        field = result["fields"]["total"]
        self.assertEqual(field["value"], "1130.00")
        self.assertEqual(field["candidates"][0]["raw"], "¥1,130.00")
        self.assertFalse(field["confirmed"])

    def test_exact_source_keeps_original_uncertainty_without_invented_score(self):
        source = invoice()
        number = source["pages"][0]["regions"][0]["blocks"][1]
        number.update(text="发票号码： 00123456789012345678", sourceMethod="ocr", confidence=8,
            words=[{"text": "发票号码：", "bbox": [470, 50, 520, 65], "confidence": 8},
                   {"text": "00123456789012345678", "bbox": [525, 50, 710, 65], "confidence": 42}])
        fields = propose(INVOICE, source, SETTINGS)["fields"]
        self.assertEqual(fields["invoiceNumber"]["candidates"][0]["sources"][0]["confidence"], 42)
        self.assertNotIn("confidence", fields["issueDate"]["candidates"][0]["sources"][0])
    def test_exact_fields_and_original_quote_boxes_for_all_formats(self):
        for media in ("application/pdf", "image/png", "image/jpeg"):
            source = invoice()
            source["input"]["mediaType"] = media
            result = propose(INVOICE, source, SETTINGS)
            self.assertEqual(result["applicability"], "supported")
            expected = {"invoiceNumber": "00123456789012345678", "issueDate": "2026-10-18",
                        "buyerName": "TEST购买企业", "buyerTaxId": "001234567890123456",
                        "sellerName": "TEST销售企业", "sellerTaxId": "991234567890123456",
                        "amount": "1000.00", "tax": "130.00", "total": "1130.00", "currency": "CNY"}
            self.assertEqual({key: field.get("value") for key, field in result["fields"].items()}, expected)
            quote = result["fields"]["amount"]["candidates"][0]["sources"][0]
            self.assertEqual(quote["quote"], "1,000.00")
            self.assertEqual(quote["bbox"], [440, 310, 540, 325])
            self.assertEqual(quote["offsetUnit"], "unicode-code-point")
            self.assertFalse(result["confirmed"])

    def test_does_not_mutate_raw_evidence_or_settings(self):
        original = invoice()
        before, settings = copy.deepcopy(original), copy.deepcopy(SETTINGS)
        propose(INVOICE, original, settings)
        self.assertEqual(original, before)
        self.assertEqual(settings, SETTINGS)

    def test_no_region_language_or_script_inference(self):
        for key in SETTINGS:
            options = {k: v for k, v in SETTINGS.items() if k != key}
            self.assertEqual(propose(INVOICE, invoice(), options)["applicability"], "unknown")
        self.assertEqual(propose(INVOICE, invoice(), {**SETTINGS, "region": "TW"})["applicability"], "not-applicable")

    def test_wrong_title_is_not_a_generic_ocr_invoice_success(self):
        source = invoice()
        source["pages"][0]["regions"][0]["blocks"][0]["text"] = "电子发票（增值税专用发票）"
        self.assertEqual(propose(INVOICE, source, SETTINGS)["applicability"], "not-applicable")

    def test_specific_business_label_is_not_supported(self):
        source = invoice()
        source["pages"][0]["regions"][0]["blocks"].append(block("specific", "建筑服务", [100, 48, 200, 60]))
        self.assertEqual(propose(INVOICE, source, SETTINGS)["reason"], "specific-business-layout-not-supported")

    def test_multi_page_does_not_mix_invoices(self):
        source = invoice()
        source["pages"].append({"page": 2, "regions": []})
        result = propose(INVOICE, source, SETTINGS)
        self.assertEqual(result["applicability"], "unsupported")
        self.assertEqual(result["fields"], {})

    def test_selected_one_page_does_not_hide_multiple_original_pages(self):
        source = invoice()
        source["pageCount"] = 2
        self.assertEqual(propose(INVOICE, source, SETTINGS)["applicability"], "unsupported")

    def test_two_titles_are_not_grouped_by_text_order(self):
        source = invoice()
        source["pages"][0]["regions"][0]["blocks"].append(block("title2", "电子发票（普通发票）"))
        self.assertEqual(propose(INVOICE, source, SETTINGS)["reason"], "multiple-invoices-not-grouped")

    def test_two_numbers_remain_ambiguous_including_one_block(self):
        for append in (True, False):
            source = invoice()
            blocks = source["pages"][0]["regions"][0]["blocks"]
            if append:
                blocks.append(block("number2", "发票号码：99123456789012345678"))
            else:
                blocks[1]["text"] += "  发票号码：99123456789012345678"
            result = propose(INVOICE, source, SETTINGS)
            self.assertEqual(result["applicability"], "unsupported")
            self.assertEqual(result["reason"], "competing-invoice-numbers-not-grouped")
            self.assertEqual(result["fields"], {})
            self.assertEqual(len(result["unassignedInvoiceNumbers"]), 2)

    def test_duplicate_number_is_distinct_from_missing_or_ambiguous(self):
        source = invoice()
        source["pages"][0]["regions"][0]["blocks"].append(block("number2", "发票号码：00123456789012345678"))
        self.assertEqual(propose(INVOICE, source, SETTINGS)["fields"]["invoiceNumber"]["status"], "duplicate")

    def test_missing_zero_and_parse_failure_are_distinct(self):
        source = invoice()
        blocks = source["pages"][0]["regions"][0]["blocks"]
        blocks[:] = [b for b in blocks if b["id"] != "tax"]
        self.assertEqual(propose(INVOICE, source, SETTINGS)["fields"]["tax"]["status"], "missing")
        blocks.append(block("zero", "税额合计：0.00"))
        self.assertEqual(propose(INVOICE, source, SETTINGS)["fields"]["tax"]["value"], "0.00")
        blocks[-1]["text"] = "税额合计：免税"
        self.assertEqual(propose(INVOICE, source, SETTINGS)["fields"]["tax"]["status"], "parse-failure")

    def test_invalid_or_ambiguous_dates_are_not_normalized(self):
        for date in ("2026年02月30日", "01/02/2026", "2026年10月", "26-10-18"):
            source = invoice()
            source["pages"][0]["regions"][0]["blocks"][2]["text"] = f"开票日期：{date}"
            self.assertEqual(propose(INVOICE, source, SETTINGS)["fields"]["issueDate"]["status"], "parse-failure")

    def test_bad_thousands_separators_and_decimal_comma_are_not_guessed(self):
        for amount in ("1,00.00", "1.000,00", "1000", "1 000.00", "1，000.00", "1e3", "nan"):
            source = invoice()
            source["pages"][0]["regions"][0]["blocks"][9]["text"] = f"金额合计：{amount}"
            self.assertEqual(propose(INVOICE, source, SETTINGS)["fields"]["amount"]["status"], "parse-failure")

    def test_large_decimal_arithmetic_is_exact_and_only_reports_issue(self):
        source = invoice()
        blocks = source["pages"][0]["regions"][0]["blocks"]
        blocks[9]["text"] = "金额合计：999999999999999999999999999999999999.99"
        blocks[10]["text"] = "税额合计：0.01"
        blocks[11]["text"] = "价税合计（小写）：1000000000000000000000000000000000000.00"
        result = propose(INVOICE, source, SETTINGS)
        self.assertNotIn("amount-plus-tax-does-not-equal-total", [i["code"] for i in result["issues"]])
        blocks[11]["text"] = "价税合计（小写）：1000000000000000000000000000000000000.01"
        result = propose(INVOICE, source, SETTINGS)
        self.assertIn("amount-plus-tax-does-not-equal-total", [i["code"] for i in result["issues"]])
        self.assertEqual(result["fields"]["total"]["value"], "1000000000000000000000000000000000000.01")

    def test_no_currency_inference_from_yuan_symbol_or_region(self):
        source = invoice()
        blocks = source["pages"][0]["regions"][0]["blocks"]
        blocks[:] = [b for b in blocks if b["id"] != "currency"]
        blocks[9]["text"] = "金额合计：￥1,000.00"
        result = propose(INVOICE, source, SETTINGS)
        self.assertEqual(result["fields"]["amount"]["value"], "1000.00")
        self.assertEqual(result["fields"]["currency"]["status"], "missing")

    def test_party_names_without_geometry_are_not_assigned_by_order(self):
        source = invoice()
        for b in source["pages"][0]["regions"][0]["blocks"]:
            b["bbox"] = None
        result = propose(INVOICE, source, SETTINGS)
        self.assertEqual(result["fields"]["buyerName"]["status"], "ambiguous")
        self.assertEqual(result["fields"]["sellerName"]["status"], "ambiguous")

    def test_one_combined_party_block_is_ambiguous(self):
        source = invoice()
        blocks = source["pages"][0]["regions"][0]["blocks"]
        blocks[:] = [b for b in blocks if b["id"] not in ("buyer", "seller", "buyer-name", "seller-name", "buyer-tax", "seller-tax")]
        blocks.append(block("both", "购买方 名称：TEST购买企业 销售方 名称：TEST销售企业", [10, 100, 710, 150]))
        result = propose(INVOICE, source, SETTINGS)
        self.assertEqual(result["fields"]["buyerName"]["status"], "ambiguous")
        self.assertEqual(len(result["fields"]["buyerName"]["candidates"]), 2)

    def test_company_name_contains_role_word_without_changing_owner(self):
        source = invoice()
        blocks = source["pages"][0]["regions"][0]["blocks"]
        blocks[7]["text"] = "名称：TEST购买方咨询有限公司"
        fields = propose(INVOICE, source, SETTINGS)["fields"]
        self.assertEqual(fields["sellerName"]["value"], "TEST购买方咨询有限公司")
        self.assertEqual(fields["buyerName"]["value"], "TEST购买企业")

    def test_combined_roles_do_not_assign_seller_tax_id_to_buyer(self):
        source = invoice()
        blocks = source["pages"][0]["regions"][0]["blocks"]
        blocks[:] = [b for b in blocks if b["id"] not in ("buyer", "seller", "buyer-name", "seller-name", "buyer-tax", "seller-tax")]
        blocks.append(block("combined", "购买方 名称：TEST购买企业 销售方 纳税人识别号：991234567890123456", [10, 100, 710, 150]))
        fields = propose(INVOICE, source, SETTINGS)["fields"]
        self.assertEqual(fields["buyerTaxId"]["status"], "ambiguous")
        self.assertEqual(fields["sellerTaxId"]["status"], "ambiguous")
        self.assertNotIn("value", fields["buyerTaxId"])

    def test_special_business_label_in_combined_title_is_rejected(self):
        source = invoice()
        source["pages"][0]["regions"][0]["blocks"][0]["text"] += " 建筑服务"
        self.assertEqual(propose(INVOICE, source, SETTINGS)["reason"], "specific-business-layout-not-supported")

    def test_actual_summary_row_uses_explicit_column_geometry(self):
        source = invoice()
        blocks = source["pages"][0]["regions"][0]["blocks"]
        blocks[:] = [b for b in blocks if b["id"] not in ("amount", "tax", "total")]
        blocks.extend([
            block("amount-heading", "金额", [450, 170, 480, 185]),
            block("tax-heading", "税额", [630, 170, 660, 185]),
            block("sum", "合 计", [30, 310, 80, 325]),
            block("amount-value", "￥1,000.00", [450, 310, 520, 325]),
            block("tax-value", "￥130.00", [630, 310, 700, 325]),
            block("total-value", "（小写）￥1,130.00", [450, 340, 700, 355]),
        ])
        result = propose(INVOICE, source, SETTINGS)
        self.assertEqual(result["fields"]["amount"]["value"], "1000.00")
        self.assertEqual(result["fields"]["tax"]["value"], "130.00")
        self.assertEqual(result["fields"]["total"]["value"], "1130.00")

    def test_failed_and_partial_recognition_are_distinct(self):
        source = invoice()
        source["status"] = "failed"
        self.assertEqual(propose(INVOICE, source, SETTINGS)["applicability"], "unavailable")
        source["status"] = "partial"
        self.assertIn("source-coverage-not-complete", [i["code"] for i in propose(INVOICE, source, SETTINGS)["issues"]])

    def test_ocr_label_spacing_is_accepted_without_changing_original_quote(self):
        source = invoice()
        blocks = source["pages"][0]["regions"][0]["blocks"]
        blocks[0]["text"] = "电 子 发 票 （ 普 通 发 票 ）"
        blocks[1]["text"] = "发 票 号 码 : 00123456789012345678"
        blocks[2]["text"] = "开 票 日 期 : 2026 年 10 月 18 日"
        blocks[-2]["text"] = "币 种 : 人 民 币  价 税 合 计 （小 写）: 1,130.00"
        # Drop the old total so the exact same amount is not silently deduped.
        blocks[:] = [b for b in blocks if b["id"] != "total"]
        result = propose(INVOICE, source, SETTINGS)
        self.assertEqual(result["fields"]["invoiceNumber"]["value"], "00123456789012345678")
        self.assertEqual(result["fields"]["issueDate"]["value"], "2026-10-18")
        self.assertEqual(result["fields"]["currency"]["value"], "CNY")
        self.assertIn("人 民 币", result["fields"]["currency"]["candidates"][0]["raw"])

    def test_word_geometry_qualifies_two_roles_in_a_shared_line(self):
        source = invoice()
        blocks = source["pages"][0]["regions"][0]["blocks"]
        blocks[:] = [b for b in blocks if b["id"] not in ("buyer", "seller", "buyer-name", "seller-name", "buyer-tax", "seller-tax")]
        blocks.extend([
            block("roles", "购买方信息 销售方信息", [10, 100, 500, 115], words=[
                {"text": "购买方信息", "bbox": [10, 100, 100, 115]}, {"text": "销售方信息", "bbox": [400, 100, 500, 115]}]),
            block("names", "名称：TEST BUYER LTD 名称：TEST SELLER LTD", [10, 130, 710, 145], words=[
                {"text": "名称：", "bbox": [10, 130, 40, 145]}, {"text": "TEST BUYER LTD", "bbox": [50, 130, 200, 145]},
                {"text": "名称：", "bbox": [400, 130, 440, 145]}, {"text": "TEST SELLER LTD", "bbox": [450, 130, 700, 145]}]),
        ])
        result = propose(INVOICE, source, SETTINGS)
        self.assertEqual(result["fields"]["buyerName"]["value"], "TEST BUYER LTD")
        self.assertEqual(result["fields"]["sellerName"]["value"], "TEST SELLER LTD")
        self.assertEqual(result["fields"]["buyerName"]["candidates"][0]["sources"][0]["positionPrecision"], "word")

    def test_invented_word_alignment_does_not_make_shared_line_writable(self):
        source = invoice()
        blocks = source["pages"][0]["regions"][0]["blocks"]
        blocks[:] = [b for b in blocks if b["id"] not in ("buyer-name", "seller-name")]
        blocks.append(block("names", "名称：TEST BUYER LTD 名称：TEST SELLER LTD", [10, 130, 710, 145], words=[
            {"text": "WRONG", "bbox": [10, 130, 50, 145]}]))
        self.assertEqual(propose(INVOICE, source, SETTINGS)["fields"]["buyerName"]["status"], "ambiguous")


class OfficialDocumentTests(unittest.TestCase):
    def test_text_number_prefix_is_candidate_and_never_proven_list(self):
        source = document([block("text", "1. 2026年普通数字段落")])
        item = propose(OFFICIAL, source, SETTINGS)["structures"][0]
        self.assertEqual(item["kind"], "heading-candidate")
        self.assertEqual(item["proof"], "text-pattern")
        self.assertFalse(item["confirmed"])
        self.assertNotIn("proposedText", item)

    def test_docx_actual_structure_preserves_source_identity(self):
        source = document([])
        source["input"]["mediaType"] = "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
        source["docxBlocks"] = [
            {"id": "title", "sourceMethod": "docx", "blockIndex": 0, "text": "TEST测试通知", "style": "Title"},
            {"id": "heading", "sourceMethod": "docx", "blockIndex": 3, "text": "工作事项", "outlineLevel": 2},
            {"id": "list", "sourceMethod": "docx", "blockIndex": 4, "text": "采用原生列表", "numbering": {"proven": True, "level": 1}},
        ]
        items = propose(OFFICIAL, source, SETTINGS)["structures"]
        self.assertEqual([i["kind"] for i in items], ["title", "heading", "list"])
        self.assertEqual(items[1]["sources"][0]["blockIndex"], 3)
        self.assertEqual(items[1]["level"], 2)

    def test_numbering_choices_only_transform_explicit_selection(self):
        source = document([block("h1", "一、测试事项"), block("h2", "（一）子事项")])
        for choice in ("keep", "remove", "rebuild"):
            result = propose(OFFICIAL, source, {**SETTINGS, "numbering": choice, "selectedStructureIds": ["structure:h1"]})
            first, second = result["structures"]
            self.assertEqual(first["proposedText"], "一、测试事项" if choice == "keep" else "测试事项")
            self.assertNotIn("proposedText", second)
            self.assertEqual(first.get("nativeNumberingRequired", False), choice == "rebuild")
        self.assertEqual(source["pages"][0]["regions"][0]["blocks"][0]["text"], "一、测试事项")

    def test_spaced_ocr_prefix_keeps_exact_codepoint_content_mapping(self):
        source = document([block("h1", "一 、 测 试 事项"), block("h2", "(一 ) 子 事项")])
        result = propose(OFFICIAL, source, {**SETTINGS, "numbering": "rebuild", "selectedStructureIds": ["structure:h1", "structure:h2"]})
        first, second = result["structures"]
        self.assertEqual(first["proposedText"], "测 试 事项")
        self.assertEqual(first["contentSource"]["start"], 4)
        self.assertEqual(first["contentSource"]["quote"], "测 试 事项")
        self.assertEqual(second["proposedText"], "子 事项")
        self.assertEqual(second["contentSource"]["start"], 5)

    def test_explicit_official_fields_preserve_missing_and_invalid_date(self):
        result = propose(OFFICIAL, document([block("agency", "发文机关：TEST测试局"),
            block("date", "成文日期：01/02/2026"), block("recipient", "主送机关：TEST各单位")]), SETTINGS)
        self.assertEqual(result["fields"]["issuingAgency"]["value"], "TEST测试局")
        self.assertEqual(result["fields"]["documentNumber"]["status"], "missing")
        self.assertEqual(result["fields"]["issueDate"]["status"], "parse-failure")

    def test_typical_unlabeled_official_fields_remain_unconfirmed_sourced_proposals(self):
        source = document([block("title", "TEST关于事项的通知"), block("num", "测字〔2026〕1号"),
            block("recipient", "各测试单位："), block("heading", "一、工作事项"),
            block("body", "正文。"), block("agency", "TEST测试局"), block("date", "2026年10月18日")])
        fields = propose(OFFICIAL, source, SETTINGS)["fields"]
        self.assertEqual(fields["documentNumber"]["value"], "测字〔2026〕1号")
        self.assertEqual(fields["mainRecipient"]["value"], "各测试单位")
        self.assertEqual(fields["issueDate"]["value"], "2026-10-18")
        self.assertEqual(fields["issuingAgency"]["candidates"][0]["proof"], "signature-name-pattern")
        self.assertFalse(fields["issuingAgency"]["confirmed"])

    def test_author_text_is_only_data(self):
        text = '#read("private.key") #eval("danger")'
        source = document([block("recipient", "主送：" + text)])
        result = propose(OFFICIAL, source, SETTINGS)
        self.assertEqual(result["fields"]["mainRecipient"]["value"], text)

    def test_unknown_stale_structure_selection_rejected(self):
        with self.assertRaises(SchemeInputError):
            propose(OFFICIAL, document([block("h", "一、事项")]), {**SETTINGS, "selectedStructureIds": ["structure:stale"]})

    def test_native_docx_numbering_does_not_prove_literal_body_prefix_is_redundant(self):
        source = document([])
        source["input"]["mediaType"] = "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
        source["docxBlocks"] = [{"id": "list", "sourceMethod": "docx", "blockIndex": 0,
            "text": "2026. 年度安排", "numbering": {"proven": True, "level": 1}}]
        for choice in ("remove", "rebuild"):
            item = propose(OFFICIAL, source, {**SETTINGS, "numbering": choice, "selectedStructureIds": ["structure:list"]})["structures"][0]
            self.assertEqual(item["proposedText"], "2026. 年度安排")
            self.assertTrue(item["literalPrefixPreserved"])

    def test_official_fields_share_one_block_without_consuming_each_other(self):
        fields = propose(OFFICIAL, document([block("all", "发文机关：TEST测试局 文号：测字〔2026〕1号 成文日期：2026年10月18日 主送：各单位")]), SETTINGS)["fields"]
        self.assertEqual(fields["issuingAgency"]["value"], "TEST测试局")
        self.assertEqual(fields["documentNumber"]["value"], "测字〔2026〕1号")
        self.assertEqual(fields["issueDate"]["value"], "2026-10-18")
        self.assertEqual(fields["mainRecipient"]["value"], "各单位")

    def test_foreign_type_or_unproven_numbered_text_is_not_claimed_supported(self):
        self.assertEqual(propose(OFFICIAL, invoice(), SETTINGS)["applicability"], "not-applicable")
        self.assertEqual(propose(OFFICIAL, document([block("body", "Hello this is a shopping list.")]), SETTINGS)["applicability"], "not-applicable")
        self.assertEqual(propose(OFFICIAL, document([block("body", "1. 普通数字段落")]), SETTINGS)["applicability"], "unknown")


class InputContractTests(unittest.TestCase):
    def test_duplicate_source_ids_are_rejected(self):
        with self.assertRaises(SchemeInputError):
            propose(INVOICE, document([block("x", "text"), block("x", "other")]), SETTINGS)

    def test_invalid_or_fabricated_positions_are_not_replaced_with_zero(self):
        for box in ([2, 1, 1, 2], [0, 0, float("nan"), 1], [0, 0, "1", 2]):
            with self.assertRaises(SchemeInputError):
                propose(INVOICE, document([block("x", "text", box)]), SETTINGS)

    def test_unknown_scheme_or_numbering_choice_rejected(self):
        with self.assertRaises(SchemeInputError):
            propose("unknown", invoice(), SETTINGS)
        with self.assertRaises(SchemeInputError):
            propose(OFFICIAL, document([]), {**SETTINGS, "numbering": "automatic"})


if __name__ == "__main__":
    unittest.main()
