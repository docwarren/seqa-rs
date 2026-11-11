# seqa_lambda

An AWS Lambda function (HTTP, via API Gateway) that runs `seqa_core` genomic searches and gene lookups.

## Routes

| Method | Path | Description |
|--------|------|-------------|
| `POST` | any path | Genomic range query — request and response below |
| `GET`  | `/genes/symbols/{genome}/{source}` | List gene symbols; `source` is `refseq` or `gencode` |
| `GET`  | `/genes/cytobands/{genome}/{chromosome}` | Cytobands for a chromosome |

`genome` is one of `hg19`, `hg38`, `grch37` or `grch38`.

### Search request body

```json
{
    "path": "s3://my-bucket/sample.bam",
    "coordinates": "chr12:10000000-10010000",
    "with_header": false,
    "only_header": false,
    "genome": "hg38"
}
```

| Field | Required | Description |
|-------|----------|-------------|
| `path` | yes | URI of the data file; the format and index path are inferred from its extension unless set below |
| `coordinates` | yes | One region, 1-based inclusive, e.g. `chr1:1000-2000` |
| `with_header` | no | Return the file header in `header`. Defaults to `false` |
| `only_header` | no | Return the header and no records. Defaults to `false` |
| `genome` | no | Genome build, defaults to `hg38`. Not currently used by the search |
| `index_path` | no | URI of the index file, if it isn't next to the data file |
| `output_format` | no | File format (`bam`, `vcf`, `bed`, `bedgraph`, `gff`, `gtf`, `fasta`, `bigwig`, `bigbed`), if it can't be inferred from the extension |

### Search response

```json
{
    "coordinates": ["chr12:10000000-10010000"],
    "header": [],
    "lines": [["...SAM record...", "..."]]
}
```

`lines` has one array per entry in `coordinates`. An array is empty when the coordinate matches nothing, names a chromosome the file doesn't contain, or is over the format's size limit. `header` is empty unless `with_header` or `only_header` is `true`.

Cloud credentials are read from the Lambda's environment — see [seqa_core/README.md](../seqa_core/README.md#environment-variables).

## Prerequisites

- [Rust](https://www.rust-lang.org/tools/install)
- [Cargo Lambda](https://www.cargo-lambda.info/guide/installation.html)

## Building

To build the project for production, run `cargo lambda build --release`. Remove the `--release` flag to build for development.

Read more about building your lambda function in [the Cargo Lambda documentation](https://www.cargo-lambda.info/commands/build.html).

## Testing

You can run regular Rust unit tests with `cargo test`.

If you want to run integration tests locally, you can use the `cargo lambda watch` and `cargo lambda invoke` commands to do it.

First, run `cargo lambda watch` to start a local server. When you make changes to the code, the server will automatically restart.

Second, you'll need a way to pass the event data to the lambda function.

You can use the existent [event payloads](https://github.com/awslabs/aws-lambda-rust-runtime/tree/main/lambda-events/src/fixtures) in the Rust Runtime repository if your lambda function is using one of the supported event types.

You can use those examples directly with the `--data-example` flag, where the value is the name of the file in the [lambda-events](https://github.com/awslabs/aws-lambda-rust-runtime/tree/main/lambda-events/src/fixtures) repository without the `example_` prefix and the `.json` extension.

```bash
cargo lambda invoke --data-example apigw-request
```

For generic events, where you define the event data structure, you can create a JSON file with the data you want to test with. For example:

```json
{
    "command": "test"
}
```

Then, run `cargo lambda invoke --data-file ./data.json` to invoke the function with the data in `data.json`.

For HTTP events, you can also call the function directly with cURL or any other HTTP client. For example:

```bash
curl https://localhost:9000
```

Read more about running the local server in [the Cargo Lambda documentation for the `watch` command](https://www.cargo-lambda.info/commands/watch.html).
Read more about invoking the function in [the Cargo Lambda documentation for the `invoke` command](https://www.cargo-lambda.info/commands/invoke.html).

## Deploying

To deploy the project, run `cargo lambda deploy`. This will create an IAM role and a Lambda function in your AWS account.

Read more about deploying your lambda function in [the Cargo Lambda documentation](https://www.cargo-lambda.info/commands/deploy.html).
