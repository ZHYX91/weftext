package org.weftext.android.host

import org.junit.Assert.*
import org.junit.Test
import java.nio.charset.CharacterCodingException

class SurfaceGuardTest {
    @Test fun actualOriginAndFrameAreRequired() {
        val guard = SurfaceGuard()
        assertTrue(guard.accepts(SurfaceGuard.ORIGIN, true))
        for (value in listOf("http://appassets.androidplatform.net", "https://appassets.androidplatform.net:443",
            "https://appassets.androidplatform.net.evil", "https://other.invalid", "null",
            "https://user@appassets.androidplatform.net", SurfaceGuard.ORIGIN + "/index.html")) {
            assertFalse(value, guard.accepts(value, true))
        }
        assertFalse(guard.accepts(SurfaceGuard.ORIGIN, false))
        guard.retire()
        assertFalse(guard.accepts(SurfaceGuard.ORIGIN, true))
        assertTrue(SurfaceGuard().current())
    }

    @Test fun traversalAndAlternateSchemesNeverReachAssets() {
        assertEquals("assets/index-abc.js", SurfaceGuard.assetPath(SurfaceGuard.ORIGIN + "/assets/index-abc.js"))
        for (path in listOf("/../secret", "/assets/./secret", "/%2e%2e/secret", "/x?path=secret", "/x#fragment",
            "/a\\b", "//server/../secret")) {
            assertNull(path, SurfaceGuard.assetPath(SurfaceGuard.ORIGIN + path))
        }
        assertNull(SurfaceGuard.assetPath("file:///data/user/0/org.weftext/secret"))
        assertNull(SurfaceGuard.assetPath("content://org.weftext/private"))
    }

    @Test fun frozenUtf8PreservesAstralCharactersAndLineEndings() {
        val original = "中文\uD83D\uDE03\r\nnext\n\u0000"
        assertEquals(original, strictText(strictUtf8(original)))
        assertArrayEquals(byteArrayOf(0x0d, 0x0a), strictUtf8("\r\n"))
    }

    @Test fun malformedUnicodeIsRejectedWithoutReplacement() {
        assertThrows(CharacterCodingException::class.java) { strictUtf8("\uD800") }
        assertThrows(CharacterCodingException::class.java) { strictText(byteArrayOf(0xc0.toByte(), 0x80.toByte())) }
    }

    @Test fun generationsNeverShareTheOldMessageObject() {
        val old = SurfaceGuard(); old.retire()
        val next = SurfaceGuard()
        assertNotEquals(old.generation, next.generation)
        assertNotEquals(old.objectName, next.objectName)
        assertFalse(old.current())
    }
}
