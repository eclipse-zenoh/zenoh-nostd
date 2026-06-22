//
// Copyright (c) 2026 Angelo Corsaro
//
// This program and the accompanying materials are made available under the
// terms of the Eclipse Public License 2.0 which is available at
// http://www.eclipse.org/legal/epl-2.0, or the Apache License, Version 2.0
// which is available at https://www.apache.org/licenses/LICENSE-2.0.
//
// SPDX-License-Identifier: EPL-2.0 OR Apache-2.0
//
// Contributors:
//   Angelo Corsaro, <kydos@protonmail.com>
//
//
/**
 * Test: Encoding constants (pure TypeScript, no network required).
 */
import { assertEquals, assertNotEquals } from "jsr:@std/assert";
import { Encoding } from "../../src/index.ts";

Deno.test("Encoding: static constants have correct IDs", () => {
    assertEquals(Encoding.ZENOH_BYTES.id, 0);
    assertEquals(Encoding.ZENOH_INT8.id, 1);
    assertEquals(Encoding.ZENOH_STRING.id, 14);
    assertEquals(Encoding.APPLICATION_OCTET_STREAM.id, 16);
    assertEquals(Encoding.TEXT_PLAIN.id, 17);
    assertEquals(Encoding.APPLICATION_JSON.id, 18);
    assertEquals(Encoding.IMAGE_PNG.id, 29);
    assertEquals(Encoding.VIDEO_WEBM.id, 71);
});

Deno.test("Encoding: DEFAULT is ZENOH_BYTES", () => {
    assertEquals(Encoding.DEFAULT.id, 0);
});

Deno.test("Encoding: withSchema returns new instance with schema", () => {
    const enc = Encoding.APPLICATION_JSON.withSchema("my-schema");
    assertEquals(enc.id, 18);
    assertEquals(enc.schema, "my-schema");
    // Original is unchanged
    assertEquals(Encoding.APPLICATION_JSON.schema, "application/json");
});

Deno.test("Encoding: toString returns schema string", () => {
    assertEquals(Encoding.TEXT_PLAIN.toString(), "text/plain");
    assertEquals(Encoding.ZENOH_BYTES.toString(), "zenoh/bytes");
});

Deno.test("Encoding: equals comparison", () => {
    assertEquals(Encoding.TEXT_PLAIN.equals(Encoding.TEXT_PLAIN), true);
    assertEquals(Encoding.TEXT_PLAIN.equals(new Encoding(17, "text/plain")), true);
    assertNotEquals(Encoding.TEXT_PLAIN.equals(Encoding.APPLICATION_JSON), true);
});

Deno.test("Encoding: all 72 encodings have unique IDs 0-71", () => {
    const encodings = [
        Encoding.ZENOH_BYTES, Encoding.ZENOH_INT8, Encoding.ZENOH_INT16,
        Encoding.ZENOH_INT32, Encoding.ZENOH_INT64, Encoding.ZENOH_INT128,
        Encoding.ZENOH_UINT8, Encoding.ZENOH_UINT16, Encoding.ZENOH_UINT32,
        Encoding.ZENOH_UINT64, Encoding.ZENOH_UINT128, Encoding.ZENOH_FLOAT32,
        Encoding.ZENOH_FLOAT64, Encoding.ZENOH_BOOL, Encoding.ZENOH_STRING,
        Encoding.ZENOH_ERROR, Encoding.APPLICATION_OCTET_STREAM, Encoding.TEXT_PLAIN,
        Encoding.APPLICATION_JSON, Encoding.TEXT_JSON, Encoding.APPLICATION_CDR,
        Encoding.APPLICATION_CBOR, Encoding.APPLICATION_YAML, Encoding.TEXT_YAML,
        Encoding.TEXT_JSON5, Encoding.APPLICATION_PYTHON_SERIALIZED_OBJECT,
        Encoding.APPLICATION_PROTOBUF, Encoding.APPLICATION_JAVA_SERIALIZED_OBJECT,
        Encoding.APPLICATION_OPENMETRICS_TEXT, Encoding.IMAGE_PNG, Encoding.IMAGE_JPEG,
        Encoding.IMAGE_GIF, Encoding.IMAGE_BMP, Encoding.IMAGE_WEBP,
        Encoding.APPLICATION_XML, Encoding.APPLICATION_X_WWW_FORM_URLENCODED,
        Encoding.TEXT_HTML, Encoding.TEXT_XML, Encoding.TEXT_CSS,
        Encoding.TEXT_JAVASCRIPT, Encoding.TEXT_MARKDOWN, Encoding.TEXT_CSV,
        Encoding.APPLICATION_SQL, Encoding.APPLICATION_COAP_PAYLOAD,
        Encoding.APPLICATION_LINKFORMAT, Encoding.APPLICATION_SENML_JSON,
        Encoding.APPLICATION_SENML_CBOR, Encoding.APPLICATION_EXI,
        Encoding.APPLICATION_FASTINFOSET, Encoding.APPLICATION_SOAP_XML,
        Encoding.APPLICATION_ATOM_XML, Encoding.APPLICATION_RSS_XML,
        Encoding.APPLICATION_EPUB_ZIP, Encoding.APPLICATION_WASM,
        Encoding.APPLICATION_JAVA_VM, Encoding.APPLICATION_JAVASCRIPT,
        Encoding.IMAGE_X_ICON, Encoding.IMAGE_SVG_XML, Encoding.IMAGE_TIFF,
        Encoding.AUDIO_FLAC, Encoding.AUDIO_AAC, Encoding.AUDIO_OGG,
        Encoding.AUDIO_MPEG, Encoding.VIDEO_H261, Encoding.VIDEO_H263,
        Encoding.VIDEO_H264, Encoding.VIDEO_H265, Encoding.VIDEO_H266,
        Encoding.VIDEO_MP4, Encoding.VIDEO_OGG, Encoding.VIDEO_MPEG,
        Encoding.VIDEO_WEBM,
    ];
    const ids = new Set(encodings.map((e) => e.id));
    assertEquals(ids.size, 72, `Expected 72 unique encoding IDs, got ${ids.size}`);
    for (let i = 0; i <= 71; i++) {
        assertEquals(ids.has(i), true, `Missing encoding ID ${i}`);
    }
});
