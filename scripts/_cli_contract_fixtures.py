"""Pure authored content for the existing real remote chunk contract."""
from dataclasses import dataclass
import hashlib


@dataclass(frozen=True)
class RemoteContractText:
    source: str
    tokens: tuple[str, ...]
    maximum_paragraph_tokens: int


def remote_contract_text(original_source: str) -> RemoteContractText:
    """Retain the original source and the fixed 2048-token/16-token layout.

    This constructs no session, basis, request, artifact or success evidence.
    The caller must still produce and verify the actual compressed DOCX/chunks.
    """
    tokens = tuple(hashlib.sha256(str(index).encode('ascii')).hexdigest()
                   for index in range(2048))
    source = original_source + '\n\n' + '\n\n'.join(
        ' '.join(tokens[index:index + 16]) for index in range(0, len(tokens), 16)) + '\n中文😀\n'
    return RemoteContractText(source, tokens, 16)
