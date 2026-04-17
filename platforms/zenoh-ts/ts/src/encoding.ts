export class Encoding {
    static readonly ZENOH_BYTES                              = new Encoding(0,  "zenoh/bytes");
    static readonly ZENOH_INT8                               = new Encoding(1,  "zenoh/int8");
    static readonly ZENOH_INT16                              = new Encoding(2,  "zenoh/int16");
    static readonly ZENOH_INT32                              = new Encoding(3,  "zenoh/int32");
    static readonly ZENOH_INT64                              = new Encoding(4,  "zenoh/int64");
    static readonly ZENOH_INT128                             = new Encoding(5,  "zenoh/int128");
    static readonly ZENOH_UINT8                              = new Encoding(6,  "zenoh/uint8");
    static readonly ZENOH_UINT16                             = new Encoding(7,  "zenoh/uint16");
    static readonly ZENOH_UINT32                             = new Encoding(8,  "zenoh/uint32");
    static readonly ZENOH_UINT64                             = new Encoding(9,  "zenoh/uint64");
    static readonly ZENOH_UINT128                            = new Encoding(10, "zenoh/uint128");
    static readonly ZENOH_FLOAT32                            = new Encoding(11, "zenoh/float32");
    static readonly ZENOH_FLOAT64                            = new Encoding(12, "zenoh/float64");
    static readonly ZENOH_BOOL                               = new Encoding(13, "zenoh/bool");
    static readonly ZENOH_STRING                             = new Encoding(14, "zenoh/string");
    static readonly ZENOH_ERROR                              = new Encoding(15, "zenoh/error");
    static readonly APPLICATION_OCTET_STREAM                 = new Encoding(16, "application/octet-stream");
    static readonly TEXT_PLAIN                               = new Encoding(17, "text/plain");
    static readonly APPLICATION_JSON                         = new Encoding(18, "application/json");
    static readonly TEXT_JSON                                = new Encoding(19, "text/json");
    static readonly APPLICATION_CDR                          = new Encoding(20, "application/cdr");
    static readonly APPLICATION_CBOR                         = new Encoding(21, "application/cbor");
    static readonly APPLICATION_YAML                         = new Encoding(22, "application/yaml");
    static readonly TEXT_YAML                                = new Encoding(23, "text/yaml");
    static readonly TEXT_JSON5                               = new Encoding(24, "text/json5");
    static readonly APPLICATION_PYTHON_SERIALIZED_OBJECT     = new Encoding(25, "application/python-serialized-object");
    static readonly APPLICATION_PROTOBUF                     = new Encoding(26, "application/protobuf");
    static readonly APPLICATION_JAVA_SERIALIZED_OBJECT       = new Encoding(27, "application/java-serialized-object");
    static readonly APPLICATION_OPENMETRICS_TEXT             = new Encoding(28, "application/openmetrics-text");
    static readonly IMAGE_PNG                                = new Encoding(29, "image/png");
    static readonly IMAGE_JPEG                               = new Encoding(30, "image/jpeg");
    static readonly IMAGE_GIF                                = new Encoding(31, "image/gif");
    static readonly IMAGE_BMP                                = new Encoding(32, "image/bmp");
    static readonly IMAGE_WEBP                               = new Encoding(33, "image/webp");
    static readonly APPLICATION_XML                          = new Encoding(34, "application/xml");
    static readonly APPLICATION_X_WWW_FORM_URLENCODED        = new Encoding(35, "application/x-www-form-urlencoded");
    static readonly TEXT_HTML                                = new Encoding(36, "text/html");
    static readonly TEXT_XML                                 = new Encoding(37, "text/xml");
    static readonly TEXT_CSS                                 = new Encoding(38, "text/css");
    static readonly TEXT_JAVASCRIPT                          = new Encoding(39, "text/javascript");
    static readonly TEXT_MARKDOWN                            = new Encoding(40, "text/markdown");
    static readonly TEXT_CSV                                 = new Encoding(41, "text/csv");
    static readonly APPLICATION_SQL                          = new Encoding(42, "application/sql");
    static readonly APPLICATION_COAP_PAYLOAD                 = new Encoding(43, "application/coap-payload");
    static readonly APPLICATION_LINKFORMAT                   = new Encoding(44, "application/link-format");
    static readonly APPLICATION_SENML_JSON                   = new Encoding(45, "application/senml+json");
    static readonly APPLICATION_SENML_CBOR                   = new Encoding(46, "application/senml+cbor");
    static readonly APPLICATION_EXI                          = new Encoding(47, "application/exi");
    static readonly APPLICATION_FASTINFOSET                  = new Encoding(48, "application/fastinfoset");
    static readonly APPLICATION_SOAP_XML                     = new Encoding(49, "application/soap+xml");
    static readonly APPLICATION_ATOM_XML                     = new Encoding(50, "application/atom+xml");
    static readonly APPLICATION_RSS_XML                      = new Encoding(51, "application/rss+xml");
    static readonly APPLICATION_EPUB_ZIP                     = new Encoding(52, "application/epub+zip");
    static readonly APPLICATION_WASM                         = new Encoding(53, "application/wasm");
    static readonly APPLICATION_JAVA_VM                      = new Encoding(54, "application/java-vm");
    static readonly APPLICATION_JAVASCRIPT                   = new Encoding(55, "application/javascript");
    static readonly IMAGE_X_ICON                             = new Encoding(56, "image/x-icon");
    static readonly IMAGE_SVG_XML                            = new Encoding(57, "image/svg+xml");
    static readonly IMAGE_TIFF                               = new Encoding(58, "image/tiff");
    static readonly AUDIO_FLAC                               = new Encoding(59, "audio/flac");
    static readonly AUDIO_AAC                                = new Encoding(60, "audio/aac");
    static readonly AUDIO_OGG                                = new Encoding(61, "audio/ogg");
    static readonly AUDIO_MPEG                               = new Encoding(62, "audio/mpeg");
    static readonly VIDEO_H261                               = new Encoding(63, "video/h261");
    static readonly VIDEO_H263                               = new Encoding(64, "video/h263");
    static readonly VIDEO_H264                               = new Encoding(65, "video/h264");
    static readonly VIDEO_H265                               = new Encoding(66, "video/h265");
    static readonly VIDEO_H266                               = new Encoding(67, "video/h266");
    static readonly VIDEO_MP4                                = new Encoding(68, "video/mp4");
    static readonly VIDEO_OGG                                = new Encoding(69, "video/ogg");
    static readonly VIDEO_MPEG                               = new Encoding(70, "video/mpeg");
    static readonly VIDEO_WEBM                               = new Encoding(71, "video/webm");

    static readonly DEFAULT = Encoding.ZENOH_BYTES;

    readonly id: number;
    readonly schema: string | undefined;

    constructor(id: number, schema?: string) {
        this.id = id;
        this.schema = schema;
    }

    withSchema(schema: string): Encoding {
        return new Encoding(this.id, schema);
    }

    toString(): string {
        return this.schema ?? String(this.id);
    }

    equals(other: Encoding): boolean {
        return this.id === other.id && this.schema === other.schema;
    }
}
