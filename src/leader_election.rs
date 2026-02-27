use k8s_openapi::api::coordination::v1::{Lease, LeaseSpec};
use k8s_openapi::apimachinery::pkg::apis::meta::v1::MicroTime;
use kube::api::{Patch, PatchParams, PostParams};
use kube::{Api, Client};
use std::env;
use tokio::task::JoinHandle;
use tokio::time::{Duration, sleep};

#[derive(Clone)]
struct LeaseManager {
    leases: Api<Lease>,
    lease_name: String,
    holder_identity: String,
    lease_duration_seconds: i32,
}

impl LeaseManager {
    async fn acquire_or_renew(&self) -> Result<bool, kube::Error> {
        let now = MicroTime(chrono::Utc::now());
        let lease = self.leases.get_opt(&self.lease_name).await?;

        if let Some(existing) = lease {
            let spec = existing.spec.unwrap_or_default();
            let duration_seconds = spec
                .lease_duration_seconds
                .unwrap_or(self.lease_duration_seconds)
                .max(1);

            let holder = spec.holder_identity.clone().unwrap_or_default();
            let expired = spec
                .renew_time
                .as_ref()
                .map(|t| {
                    chrono::Utc::now().signed_duration_since(t.0).num_seconds()
                        >= i64::from(duration_seconds)
                })
                .unwrap_or(true);

            if holder == self.holder_identity || expired {
                let patch = serde_json::json!({
                    "spec": {
                        "holderIdentity": self.holder_identity,
                        "leaseDurationSeconds": duration_seconds,
                        "renewTime": now,
                        "acquireTime": spec.acquire_time.unwrap_or(now),
                    }
                });

                let _ = self
                    .leases
                    .patch(
                        &self.lease_name,
                        &PatchParams::default(),
                        &Patch::Merge(&patch),
                    )
                    .await?;
                return Ok(true);
            }

            return Ok(false);
        }

        let create = Lease {
            metadata: kube::core::ObjectMeta {
                name: Some(self.lease_name.clone()),
                ..Default::default()
            },
            spec: Some(LeaseSpec {
                holder_identity: Some(self.holder_identity.clone()),
                lease_duration_seconds: Some(self.lease_duration_seconds),
                acquire_time: Some(now.clone()),
                renew_time: Some(now),
                ..Default::default()
            }),
        };

        match self.leases.create(&PostParams::default(), &create).await {
            Ok(_) => Ok(true),
            Err(kube::Error::Api(ae)) if ae.code == 409 => Ok(false),
            Err(err) => Err(err),
        }
    }
}

pub struct LeaderGuard {
    renew_task: JoinHandle<()>,
}

impl Drop for LeaderGuard {
    fn drop(&mut self) {
        self.renew_task.abort();
    }
}

pub async fn try_acquire_leadership() -> Result<Option<LeaderGuard>, String> {
    let enabled = env::var("K8S_LEADER_ELECTION")
        .map(|v| matches!(v.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"))
        .unwrap_or(false);

    if !enabled {
        return Ok(None);
    }

    let namespace = env::var("POD_NAMESPACE").unwrap_or_else(|_| "default".to_string());
    let holder_identity = env::var("POD_NAME")
        .or_else(|_| env::var("HOSTNAME"))
        .unwrap_or_else(|_| "class-chat-bot".to_string());
    let lease_name =
        env::var("LEADER_ELECTION_LEASE_NAME").unwrap_or_else(|_| "class-chat-bot-discord".into());
    let lease_duration_seconds = env::var("LEADER_ELECTION_LEASE_SECONDS")
        .ok()
        .and_then(|v| v.parse::<i32>().ok())
        .unwrap_or(15)
        .max(5);

    let client = Client::try_default()
        .await
        .map_err(|err| format!("k8s client init failed: {err}"))?;
    let manager = LeaseManager {
        leases: Api::namespaced(client, &namespace),
        lease_name,
        holder_identity,
        lease_duration_seconds,
    };

    loop {
        match manager.acquire_or_renew().await {
            Ok(true) => break,
            Ok(false) => sleep(Duration::from_secs(2)).await,
            Err(err) => {
                eprintln!("[leader-election] acquire failed: {err}");
                sleep(Duration::from_secs(2)).await;
            }
        }
    }

    let renewer = manager.clone();
    let renew_task = tokio::spawn(async move {
        let interval = (renewer.lease_duration_seconds / 3).max(1) as u64;
        loop {
            sleep(Duration::from_secs(interval)).await;
            match renewer.acquire_or_renew().await {
                Ok(true) => {}
                Ok(false) => {
                    eprintln!("[leader-election] leadership lost, shutting down");
                    std::process::exit(1);
                }
                Err(err) => {
                    eprintln!("[leader-election] renew failed: {err}");
                }
            }
        }
    });

    Ok(Some(LeaderGuard { renew_task }))
}
