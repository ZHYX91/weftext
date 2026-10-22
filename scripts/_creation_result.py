"""Test-fixture projection of the current commit/delivery response contract."""
def created_document(envelope, operation_id=None):
    assert envelope.get('status') == 'committed', envelope
    assert isinstance(envelope.get('operationId'), str), envelope
    if operation_id is not None:
        assert envelope['operationId'] == operation_id, envelope
    assert not envelope.get('documentError'), envelope
    document = envelope.get('document')
    assert isinstance(document, dict) and isinstance(document.get('sessionId'), str), envelope
    return document


def library_fact(envelope, operation_id=None):
    """Original LibraryInfo only; independently renewed delivery is not fact."""
    assert envelope.get('status') == 'committed', envelope
    if operation_id is not None:
        assert envelope.get('operationId') == operation_id, envelope
    assert all(isinstance(envelope.get(key), str) and envelope[key]
               for key in ('operationId', 'root', 'workspaceId')), envelope
    return {key: envelope[key] for key in
            ('operationId', 'root', 'workspaceId', 'status', 'rootNotePath') if key in envelope}
