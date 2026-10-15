//! Standalone mock of the spec-provider for developing the spec-viewer by hand.
//!
//! Run with `cargo run --example mock_provider`, then start the spec-viewer with
//! `SPEC_PROVIDER_URL=http://localhost:50051` (the default in `.env`).
//!
//! Override the listen address with `MOCK_PROVIDER_ADDR`.

use prost_types::Timestamp;
use spec_api::{
    Draft, DraftVisibility, GetSpecRequest, GetSpecResponse, Patch, Proposal, ProposalStatus, Spec,
    Version,
    get_spec_response::Kind,
    spec_provider_server::{SpecProvider, SpecProviderServer},
};
use tonic::{Request, Response, Status};

// Known ids, anything else is NOT_FOUND.
/// A published spec.
const PUBLISHED_ID: &str = "b790c2e4-235e-49ce-b209-9755abd42538";
/// A draft with versions, an unversioned patch and proposals.
const DRAFT_ID: &str = "3f8a6d1e-5c42-4b7a-9e0d-2a1c7f6b8e34";
/// A response with neither spec nor draft.
const EMPTY_ID: &str = "c5d2e9a7-18b3-4f60-a4d1-7e93b0c2f815";

const DEFAULT_ADDR: &str = "127.0.0.1:50051";

struct MockProvider;

#[tonic::async_trait]
impl SpecProvider for MockProvider {
    async fn get_spec(
        &self,
        request: Request<GetSpecRequest>,
    ) -> Result<Response<GetSpecResponse>, Status> {
        let id = request.into_inner().id;
        println!("get_spec({id})");

        let kind = match id.as_str() {
            PUBLISHED_ID => Some(Kind::Published(published_spec(id))),
            DRAFT_ID => Some(Kind::Draft(draft_spec(id))),
            EMPTY_ID => None,
            _ => return Err(Status::not_found("no such spec")),
        };
        Ok(Response::new(GetSpecResponse { kind }))
    }
}

fn published_spec(id: String) -> Spec {
    Spec {
        id,
        title: "Example Spec".into(),
        abbreviation: "EX".into(),
        versions: vec![
            Version {
                id: "v1".into(),
                patches: vec![accepted_patch("patch-1", 1_700_000_000)],
            },
            Version {
                id: "v2".into(),
                patches: vec![
                    accepted_patch("patch-2", 1_710_000_000),
                    accepted_patch("patch-3", 1_720_000_000),
                ],
            },
        ],
    }
}

fn draft_spec(id: String) -> Draft {
    Draft {
        id,
        title: "Example Draft".into(),
        abbreviation: "EXD".into(),
        visibility: DraftVisibility::Public as i32,
        versions: vec![Version {
            id: "v1".into(),
            patches: vec![accepted_patch("patch-1", 1_700_000_000)],
        }],
        unversioned_patches: vec![accepted_patch("patch-2", 1_710_000_000)],
        proposals: vec![
            Proposal {
                id: "proposal-1".into(),
                proposed_by: "alice".into(),
                created_at: Some(timestamp(1_715_000_000)),
                last_updated_at: Some(timestamp(1_716_000_000)),
                status: ProposalStatus::Draft as i32,
                content_uri: "https://example.com/proposals/1".into(),
            },
            Proposal {
                id: "proposal-2".into(),
                proposed_by: "bob".into(),
                created_at: Some(timestamp(1_717_000_000)),
                last_updated_at: Some(timestamp(1_718_000_000)),
                status: ProposalStatus::Ready as i32,
                content_uri: "https://example.com/proposals/2".into(),
            },
        ],
    }
}

fn accepted_patch(id: &str, seconds: i64) -> Patch {
    Patch {
        id: id.into(),
        proposed_by: "alice".into(),
        created_at: Some(timestamp(seconds)),
        accepted_by: "bob".into(),
        accepted_at: Some(timestamp(seconds + 3_600)),
        content_uri: format!("https://example.com/patches/{id}"),
    }
}

fn timestamp(seconds: i64) -> Timestamp {
    Timestamp { seconds, nanos: 0 }
}

#[tokio::main]
async fn main() {
    let addr = std::env::var("MOCK_PROVIDER_ADDR")
        .unwrap_or_else(|_| DEFAULT_ADDR.into())
        .parse()
        .expect("MOCK_PROVIDER_ADDR must be a socket address like 127.0.0.1:50051");

    println!("mock spec-provider listening on {addr}");
    if let Err(error) = tonic::transport::Server::builder()
        .add_service(SpecProviderServer::new(MockProvider))
        .serve(addr)
        .await
    {
        eprintln!("mock spec-provider stopped unexpectedly: {error}");
        std::process::exit(1);
    }
}
