---
id: FR-011
title: "Versioned graph-quality observation schema"
type: FR
object: data_schema
relationships:
  - target: "ix://agent-ix/quire-code-rs/US-004"
    type: "implements"
  - target: "ix://agent-ix/quire-code-rs/FR-010"
    type: "references"
---

# [FR-011] Versioned graph-quality observation schema

## Description

A graph-quality observation SHALL conform to the engine-agnostic JSON Schema
below so that population state, raw quality results, and producing revisions are
independently validatable by assurance consumers.

## Schema

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://agent-ix.github.io/quire-code-rs/schemas/graph-quality-observation-v1.schema.json",
  "title": "GraphQualityObservationV1",
  "type": "object",
  "required": [
    "schema_version",
    "record_type",
    "observation_id",
    "producer",
    "measurement_plan",
    "population",
    "raw_scorer_output"
  ],
  "properties": {
    "schema_version": { "const": 1 },
    "record_type": { "const": "graph_quality_observation" },
    "observation_id": { "$ref": "#/$defs/digest" },
    "producer": {
      "type": "object",
      "required": [
        "extractor_revision",
        "producer_contract_version",
        "parser_grammars",
        "configuration_digest",
        "source_revision",
        "corpus_revision",
        "scorer_version"
      ],
      "properties": {
        "extractor_revision": { "$ref": "#/$defs/revision" },
        "producer_contract_version": { "type": "integer", "minimum": 1 },
        "parser_grammars": {
          "type": "array",
          "minItems": 1,
          "uniqueItems": true,
          "items": {
            "type": "object",
            "required": ["language", "grammar", "revision"],
            "properties": {
              "language": {
                "type": "string",
                "enum": ["rust", "typescript", "tsx", "python"]
              },
              "grammar": { "type": "string", "minLength": 1 },
              "revision": { "$ref": "#/$defs/revision" }
            },
            "additionalProperties": false
          }
        },
        "configuration_digest": { "$ref": "#/$defs/digest" },
        "source_revision": { "$ref": "#/$defs/revision" },
        "corpus_revision": { "$ref": "#/$defs/revision" },
        "scorer_version": { "$ref": "#/$defs/revision" }
      },
      "additionalProperties": false
    },
    "measurement_plan": {
      "type": "object",
      "required": ["ref", "definition_version"],
      "properties": {
        "ref": {
          "const": "ix://agent-ix/quire-code-rs/MP-001"
        },
        "definition_version": {
          "const": "quire-code.graph-quality-v1"
        }
      },
      "additionalProperties": false
    },
    "population": {
      "type": "object",
      "required": [
        "state",
        "files_seen",
        "supported_files",
        "unreadable_files",
        "unsupported_files",
        "census"
      ],
      "properties": {
        "state": {
          "type": "string",
          "enum": ["measured", "empty", "unreadable", "unsupported"]
        },
        "files_seen": { "type": "integer", "minimum": 0 },
        "supported_files": { "type": "integer", "minimum": 0 },
        "unreadable_files": { "type": "integer", "minimum": 0 },
        "unsupported_files": { "type": "integer", "minimum": 0 },
        "census": {
          "type": "object",
          "required": [
            "languages",
            "node_kinds",
            "relation_kinds",
            "resolver_tiers"
          ],
          "properties": {
            "languages": {
              "type": "array",
              "items": { "$ref": "#/$defs/census_item" }
            },
            "node_kinds": {
              "type": "array",
              "items": { "$ref": "#/$defs/census_item" }
            },
            "relation_kinds": {
              "type": "array",
              "items": { "$ref": "#/$defs/census_item" }
            },
            "resolver_tiers": {
              "type": "array",
              "items": { "$ref": "#/$defs/census_item" }
            }
          },
          "additionalProperties": false
        }
      },
      "additionalProperties": false
    },
    "results": {
      "type": "object",
      "required": ["confusion_matrices", "unresolved", "ambiguous", "recall"],
      "properties": {
        "confusion_matrices": {
          "type": "array",
          "minItems": 4,
          "items": { "$ref": "#/$defs/confusion_matrix" }
        },
        "unresolved": {
          "type": "array",
          "items": { "$ref": "#/$defs/dimension_count" }
        },
        "ambiguous": {
          "type": "array",
          "items": { "$ref": "#/$defs/dimension_count" }
        },
        "recall": {
          "type": "array",
          "minItems": 1,
          "items": {
            "allOf": [
              { "$ref": "#/$defs/dimension_key" },
              {
                "type": "object",
                "required": ["recovered", "expected", "ratio"],
                "properties": {
                  "recovered": { "type": "integer", "minimum": 0 },
                  "expected": { "type": "integer", "minimum": 1 },
                  "ratio": { "type": "number", "minimum": 0, "maximum": 1 }
                }
              }
            ]
          }
        }
      },
      "additionalProperties": false
    },
    "raw_scorer_output": {
      "type": "object",
      "required": ["path", "digest"],
      "properties": {
        "path": {
          "type": "string",
          "minLength": 1,
          "not": { "pattern": "^(?:/|[A-Za-z]:)" }
        },
        "digest": { "$ref": "#/$defs/digest" }
      },
      "additionalProperties": false
    }
  },
  "additionalProperties": false,
  "allOf": [
    {
      "if": {
        "properties": {
          "population": {
            "properties": { "state": { "const": "measured" } },
            "required": ["state"]
          }
        },
        "required": ["population"]
      },
      "then": {
        "required": ["results"],
        "properties": {
          "population": {
            "properties": {
              "supported_files": { "minimum": 1 },
              "unreadable_files": { "const": 0 }
            }
          }
        }
      },
      "else": { "not": { "required": ["results"] } }
    },
    {
      "if": {
        "properties": {
          "population": {
            "properties": { "state": { "const": "empty" } },
            "required": ["state"]
          }
        },
        "required": ["population"]
      },
      "then": {
        "properties": {
          "population": {
            "properties": {
              "files_seen": { "const": 0 },
              "supported_files": { "const": 0 },
              "unreadable_files": { "const": 0 },
              "unsupported_files": { "const": 0 }
            }
          }
        }
      }
    },
    {
      "if": {
        "properties": {
          "population": {
            "properties": { "state": { "const": "unreadable" } },
            "required": ["state"]
          }
        },
        "required": ["population"]
      },
      "then": {
        "properties": {
          "population": {
            "properties": { "unreadable_files": { "minimum": 1 } }
          }
        }
      }
    },
    {
      "if": {
        "properties": {
          "population": {
            "properties": { "state": { "const": "unsupported" } },
            "required": ["state"]
          }
        },
        "required": ["population"]
      },
      "then": {
        "properties": {
          "population": {
            "properties": {
              "files_seen": { "minimum": 1 },
              "supported_files": { "const": 0 },
              "unreadable_files": { "const": 0 },
              "unsupported_files": { "minimum": 1 }
            }
          }
        }
      }
    }
  ],
  "$defs": {
    "digest": {
      "type": "string",
      "pattern": "^sha256:[a-f0-9]{64}$"
    },
    "revision": {
      "type": "string",
      "pattern": "^(?:[a-f0-9]{40}|v?[0-9]+[.][0-9]+[.][0-9]+(?:[-+][0-9A-Za-z.-]+)?|sha256:[a-f0-9]{64})$"
    },
    "dimension": {
      "type": "string",
      "enum": ["overall", "language", "node_kind", "relation_kind", "resolver_tier"]
    },
    "dimension_key": {
      "type": "object",
      "required": ["dimension", "key"],
      "properties": {
        "dimension": { "$ref": "#/$defs/dimension" },
        "key": { "type": "string", "minLength": 1 }
      }
    },
    "census_item": {
      "type": "object",
      "required": ["key", "count"],
      "properties": {
        "key": { "type": "string", "minLength": 1 },
        "count": { "type": "integer", "minimum": 0 }
      },
      "additionalProperties": false
    },
    "dimension_count": {
      "allOf": [
        { "$ref": "#/$defs/dimension_key" },
        {
          "type": "object",
          "required": ["count"],
          "properties": { "count": { "type": "integer", "minimum": 0 } }
        }
      ]
    },
    "confusion_matrix": {
      "allOf": [
        { "$ref": "#/$defs/dimension_key" },
        {
          "type": "object",
          "required": [
            "true_positive",
            "false_positive",
            "false_negative",
            "true_negative"
          ],
          "properties": {
            "true_positive": { "type": "integer", "minimum": 0 },
            "false_positive": { "type": "integer", "minimum": 0 },
            "false_negative": { "type": "integer", "minimum": 0 },
            "true_negative": { "type": "integer", "minimum": 0 }
          }
        }
      ]
    }
  }
}
```

## Behavior

- A measured record SHALL contain one complete population census and a results
  block.
- A non-measured record SHALL omit the results block.
- The producer revision tuple SHALL identify the extractor, producer contract,
  parser grammars, configuration, source, corpus, and scorer.
- Each dimension array SHALL be sorted by dimension and key without duplicates.
- The observation identifier SHALL be the SHA-256 digest of the canonical record
  with `observation_id` omitted.

## Constraints

| ID | Constraint | Type | Validation |
|----|------------|------|------------|
| FR-011-CON-1 | The record SHALL remain engine-agnostic JSON data. | Compatibility | Static Test |
| FR-011-CON-2 | The record SHALL require no extractor library to parse or validate it. | Compatibility | Static Test |

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-011-AC-1 | A measured record with a supported file and complete results validates against schema version 1. | Test (TC-112) |
| FR-011-AC-2 | A measured record contains census and result entries for language, node kind, relation kind, and resolver tier plus an overall result. | Test (TC-113) |
| FR-011-AC-3 | `empty`, `unreadable`, and `unsupported` records reject a results block, while `measured` rejects its absence. | Test (TC-114) |
| FR-011-AC-4 | Missing or malformed producer revisions, grammar identities, configuration digest, or measurement-plan identity fail validation. | Test (TC-115) |
| FR-011-AC-5 | Every record retains a relative raw-output path and SHA-256 digest; an absolute path or malformed digest fails validation. | Test (TC-116) |
| FR-011-AC-6 | Unknown fields, dimension names, population states, or language names fail validation. | Test (TC-117) |

## Dependencies

- **Upstream**: [US-004](../usecase/US-004-assess-versioned-extractor-quality.md)
  and [FR-010](./FR-010-producer-invocation.md).
- **Downstream**: [FR-012](./FR-012-governed-graph-quality-producer.md) emits this
  record, and Quoin consumes it without extractor-specific code.
