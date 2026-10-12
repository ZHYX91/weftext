#metadata((id: "a9201319-d7ea-4ab1-9b89-401c8818d175", types: ())) <weftext>
#metadata((kind: "weftext.query", key: "6b2b9e19-5599-48c3-b27b-6b006ec9c397",
binding: (schema: "weftext-config://2fd54310-65f7-4d3c-8668-2f1e344a56c0/schema", revision: "6d446e07-5bfd-47c8-8f1f-307df7ee6a0f", language: "weftext.query.cel.v1"),
scope: (library: "current", lifecycle: "live"), types: ("person",),
where: "isAbsent(phones) ? false : phones.exists(p, has(p.label) && p.label == 'work')",
select: ((name: "contacts", expr: "phones.filter(p, has(p.label) && p.label == 'work')"),),
)) <weftext-query-work-phones>
此key/config revision是虚构实验标识，未获得Core出生或schema安装资格。
