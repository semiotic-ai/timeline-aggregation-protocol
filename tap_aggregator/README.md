# TAP Aggregator

A stateless JSON-RPC service that lets clients request an aggregate receipt from a list of individual receipts.

## Settings

```txt
A JSON-RPC service for the Timeline Aggregation Protocol that lets clients request an aggregate receipt from a list of
individual receipts.

Usage: tap_aggregator [OPTIONS] --private-key <PRIVATE_KEY>

Options:
      --port <PORT>
          Port to listen on for JSON-RPC requests [env: TAP_PORT=] [default: 8080]
      --private-key <PRIVATE_KEY>
          Sender private key for signing Receipt Aggregate Vouchers, as a hex string [env: TAP_PRIVATE_KEY=]
      --max-request-body-size <MAX_REQUEST_BODY_SIZE>
          Maximum request body size in bytes. Defaults to 10MB [env: TAP_MAX_REQUEST_BODY_SIZE=] [default: 10485760]
      --max-response-body-size <MAX_RESPONSE_BODY_SIZE>
          Maximum response body size in bytes. Defaults to 100kB [env: TAP_MAX_RESPONSE_BODY_SIZE=] [default: 102400]
      --max-connections <MAX_CONNECTIONS>
          Maximum number of concurrent connections. Defaults to 32 [env: TAP_MAX_CONNECTIONS=] [default: 32]
  -h, --help
          Print help
  -V, --version
          Print version
```

Please refer to
[timeline-aggregation-protocol-contracts](https://github.com/semiotic-ai/timeline-aggregation-protocol-contracts) for
more information about Receipt Aggregate Voucher signing keys.

## Operational recommendations

This is just meant to be a non-exhaustive list of reminders for safely operating the TAP Aggregator. It being an HTTP
service, use your best judgement and apply the industry-standard best practices when serving HTTP to the public
internet.

- Advertise through a safe DNS service (w/ DNSSEC, etc)
- Expose through HTTPS only (by reverse-proxying)
- Use a WAF, to leverage (if available):
  - DDoS protection, rate limiting, etc.
  - Geofencing, depending on the operator's jurisdiction.
  - HTTP response inspection.
  - JSON request and response inspection. To validate the inputs, as well as parse JSON-RPC error codes in the response.

It is also recommended that clients use HTTP compression for their HTTP requests to the TAP Aggregator, as RAV requests
can be quite large.

## JSON-RPC API

### Common interface

#### Request format

The request format is standard, as described in
[the official spec](https://www.jsonrpc.org/specification#request_object).

#### Successful response format

If the call is successful, the response format is as described in
[the official spec](https://www.jsonrpc.org/specification#response_object), and in addition the `result` field is of the
form:

```json
{
    "id": 0,
    "jsonrpc": "2.0",
    "result": {
        "data": {...},
        "warnings": [
            {
                "code": -32000,
                "message": "Error message",
                "data": {...}
            }
        ]
    }
}
```

| Field         | Type      | Description                                                                                              |
| ------------- | --------- | -------------------------------------------------------------------------------------------------------- |
| `data`        | `Object`  | The response data. Method specific, see each method's documentation.                                     |
| `warnings`    | `Array`   | (Optional) A list of warnings. If the list is empty, no warning field is added to the JSON-RPC response. |

WARNING: Always check for warnings!

Warning object format (similar to the standard JSON-RPC error object):

| Field         | Type      | Description                                                                                      |
| ------------- | --------- | ------------------------------------------------------------------------------------------------ |
| `code`        | `Integer` | A number that indicates the error type that occurred.                                            |
| `message`     | `String`  | A short description of the error.                                                                |
| `data`        | `Object`  | (Optional) A primitive or structured value that contains additional information about the error. |

We define these warning codes:

- `-32051` API version deprecation

  Also returns an object containing the method's supported versions in the `data` field. Example:

  ```json
  {
      "id": 0,
      "jsonrpc": "2.0",
      "result": {
          "data": {...},
          "warnings": [
              {
                  "code": -32051,
                  "data": {
                      "versions_deprecated": [
                          "0.0"
                      ],
                      "versions_supported": [
                          "0.0",
                          "0.1"
                      ]
                  },
                  "message": "The API version 0.0 will be deprecated. Please check https://github.com/semiotic-ai/timeline_aggregation_protocol for more information."
              }
          ]
      }
  }
  ```

#### Error response format

If the call fails, the error response format is as described in
[the official spec](https://www.jsonrpc.org/specification#error_object).

In addition to the official spec, we define a few special errors:

- `-32001` Invalid API version.

  Also returns an object containing the method's supported versions in the `data` field. Example:

  ```json
  {
      "error": {
          "code": -32001,
          "data": {
              "versions_deprecated": [
                  "0.0"
              ],
              "versions_supported": [
                  "0.0",
                  "0.1"
              ]
          },
          "message": "Unsupported API version: \"0.2\"."
      },
      "id": 0,
      "jsonrpc": "2.0"
  }
  ```

- `-32002` Aggregation error.

  The aggregation function returned an error. Example:

  ```json
  {
      "error": {
          "code": -32002,
          "message": "Signature verification failed. Expected 0x9858…da94, got 0x3ef9…a4a3"
      },
      "id": 0,
      "jsonrpc": "2.0"
  }
  ```

### Methods

#### `api_versions()`

[source](server::RpcServer::api_versions)

Returns the versions of the TAP JSON-RPC API implemented by this server.

Example:

*Request*:

```json
{
    "jsonrpc": "2.0",
    "id": 0,
    "method": "api_versions",
    "params": [
        null
    ]
}
```

*Response*:

```json
{
    "id": 0,
    "jsonrpc": "2.0",
    "result": {
        "data": {
            "versions_deprecated": [
               "0.0"
            ],
            "versions_supported": [
                "0.0",
                "0.1"
            ]
        }
    }
}
```

#### `aggregate_receipts(api_version, receipts, previous_rav)`

[source](server::RpcServer::aggregate_receipts)

Aggregates the given receipts into a receipt aggregate voucher.
Returns an error if the user expected API version is not supported.

We recommend that the server is set-up to support a maximum HTTP request size of 10MB, in which case we guarantee that
`aggregate_receipts` support a maximum of at least 15,000 receipts per call. If you have more than 15,000 receipts to
aggregate, we recommend calling `aggregate_receipts` multiple times.

Example:

*Request*:

```json
{
  "jsonrpc": "2.0",
  "id": 0,
  "method": "aggregate_receipts",
  "params": [
    "0.0",
    [
      {
        "message": {
          "allocation_id": "0xabababababababababababababababababababab",
          "timestamp_ns": 1685670449225087255,
          "nonce": 11835827017881841442,
          "value": 34
        },
        "signature": {
          "r": "0xa9fa1acf3cc3be503612f75602e68cc22286592db1f4f944c78397cbe529353b",
          "s": "0x566cfeb7e80a393021a443d5846c0734d25bcf54ed90d97effe93b1c8aef0911",
          "v": 27
        }
      },
      {
        "message": {
          "allocation_id": "0xabababababababababababababababababababab",
          "timestamp_ns": 1685670449225830106,
          "nonce": 17711980309995246801,
          "value": 23
        },
        "signature": {
          "r": "0x51ca5a2b839558654326d3a3f544a97d94effb9a7dd9cac7492007bc974e91f0",
          "s": "0x3d9d398ea6b0dd9fac97726f51c0840b8b314821fb4534cb40383850c431fd9e",
          "v": 28
        }
      }
    ],
    {
      "message": {
        "allocation_id": "0xabababababababababababababababababababab",
        "timestamp_ns": 1685670449224324338,
        "value_aggregate": 101
      },
      "signature": {
        "r": "0x601a1f399cf6223d1414a89b7bbc90ee13eeeec006bd59e0c96042266c6ad7dc",
        "s": "0x3172e795bd190865afac82e3a8be5f4ccd4b65958529986c779833625875f0b2",
        "v": 28
      }
    }
  ]
}
```

*Response*:

```json
{
  "id": 0,
  "jsonrpc": "2.0",
  "result": {
    "data": {
      "message": {
        "allocation_id": "0xabababababababababababababababababababab",
        "timestamp_ns": 1685670449225830106,
        "value_aggregate": 158
      },
      "signature": {
        "r": "0x60eb38374119bbabf1ac6960f532124ba2a9c5990d9fb50875b512e611847eb5",
        "s": "0x1b9a330cc9e2ecbda340a4757afaee8f55b6dbf278428f8cf49dd5ad8438f83d",
        "v": 27
      }
    }
  }
}
```
