# Demo Upload Large File to Azure Blob Storage
Some time when upload large file via web interface may cause blocked by WAF (Web Application Firewalls) 
because WAF limit the maximum size of HTTP request. so we need to split the large file into small pieces (chunk) and upload them one by one. 
This demo show how to upload large file to Azure Blob Storage via Web Application developed by Rust + Actix-Web.

## My Design
- Provide 3 apis for upload large file
  - `POST /api/v1/start_upload` : start and prepare cache to upload to Azure Blob Storage
  - `POST /api/v1/continue_upload` : upload each chunk to Azure Blob Storage
  - `POST /api/v1/finish_upload` : finish and clean up cache

## How to setup pre-requisites
- Install Rust
- Config Azure Service via Azure Portal
  - Provide Azure Storage Account
  - Create Container in Azure Storage Account
  - Generate application under App Registeration (in-case local development) if run on azure , prefer to use managed identity
  - Go Azure Storage Account -> Access Control (IAM) -> Add Role Assignment -> Add Storage Blob Data Contributor to application or managed identity

## How to run in local development (my mac)
use `cargo run` to run the application

```
STORAGE_ACCOUNT=<<storage account>> \
STORAGE_CONTAINER=<<container or root folder>> \
AZURE_CLIENT_ID=<<client id or application id>> \
AZURE_CLIENT_SECRET=<<client secret>> \
AZURE_TENANT_ID=<<tenant id>> \
RUST_LOG=debug cargo run

```