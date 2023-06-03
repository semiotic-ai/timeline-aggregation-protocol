# TAP Aggregator

A JSON-RPC service that lets clients request an aggregate receipt from a list of individual receipts.

## JSON-RPC API

### Common interface

The request format is standard, as described in [the official spec](https://www.jsonrpc.org/specification#request_object).

If the call is successful, the response format is as described in [the official spec](https://www.jsonrpc.org/specification#response_object).
In particular, the `result` field is of the form:

| Field         | Type      | Description                                                                                              |
| ------------- | --------- | -------------------------------------------------------------------------------------------------------- |
| `data`        | `Object`  | The response data. Method specific, see each method's documentation.                                     |
| `warnings`    | `Array`   | (Optional) A list of warnings. If the list is empty, no warning field is added to the JSON-RPC response. |

WARNING: Always check for warnings!

Warning object format (similar to the standard JSON-RPC error object):

| Field         | Type      | Description                                                                           |
| ------------- | --------- | ------------------------------------------------------------------------------------- |
| `code`        | `Integer` | A number that indicates the error type that occurred.                                 |
| `message`     | `String`  | A short description of the error.                                                     |
| `data`        | `Object`  | A primitive or structured value that contains additional information about the error. |

If the call fails, the response format is as described in [the official spec](https://www.jsonrpc.org/specification#error_object).

### Methods

#### `api_versions`

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

#### `aggregate_receipts`

[source](server::RpcServer::aggregate_receipts)

Aggregates the given receipts into a receipt aggregate voucher.
Returns an error if the user expected API version is not supported.

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
