---
source_language: zh-CN
translation_of: D7-IMPACT.zh-CN.md
translation_status: synced
---

[简体中文](D7-IMPACT.zh-CN.md)
# A2 D7 Implementation Impact and Acceptance Overlay

Status: design impact for the D7 author candidate. No implementation or product behavior is claimed.

## 1. Retained implementation surface

The complete retained implementation/test outline is preserved byte-for-byte at d7/owners/implementation-impact. Current code work must still implement the same one-Core Query DAG, CEL evaluator, result paging/subscriptions, View projection, Action prepare/preview/submit, EffectBytes delivery, Definition Transfer, Narrow Field qualification, and historical dispatch.

## 2. Current successor deltas

Fresh work additionally consumes the fixed97 current PAB4/Descriptor3/Proof3/PreparedIntent3, Action prepare/input version 3, ActionSpec2, D7ProposedInput3, EffectManifest3/EffectBytes3, D3 wire13, Notice3/CP4/ChangeRecord1, current D2DocumentSnapshot/3, Value4 Annotation, and role-specific source plans. No predecessor decoder is widened in place.

## 3. Search implementation boundary

Search UI must compile ordinary text, visual filters and explicit shortcut mode to the same Query planner. An index provider may only return candidates and coverage evidence; Core rereads authorized current values before match/rank and requires real query_scan for complete results. Search parser errors never fall back to another Query.

D8 later owns the concrete input widgets, focus model, IME adapters, mobile sheet and assistive-technology evidence. This batch freezes semantics only.

## 4. Mechanical and semantic checks

Repository checks must cover paired documentation, protected design inputs, JSON validity, D7 machine-map references, Registry byte reuse, and exact unchanged S inputs. Design-level contract checks must separately cover DAG canonicalization, all operators, late errors, arbitrary numbers, D2 full product traversal, SearchContribution activation/currentness, Narrow Field hidden-scope negatives, PAB4 freezing, Definition Transfer two-pass mapping, result reset/subscription atomicity, View pure presentation, EffectBytes authorization, and historical replay.

## 5. Product evidence boundary

Runtime, OS, GUI, IME, accessibility, renderer/export, database, real-replica, crash, performance, provider, migration and activation scenarios remain UNRUN. Documentation or source-map checks prove repository consistency only. They cannot close D7 semantics or SEARCH acceptance.

## 6. Next gates

First gate is a fixed-SHA non-author D7 review over this complete candidate. Any finding is repaired by the author without self-closing it. D8 through D10 remain later full-module batches, followed by a fresh global non-author review and an explicit accepted design SHA.
