use crate::testing::fragments::node::FragmentNode;
use crate::testing::fragments::node::FragmentNodeError;
use crate::testing::fragments::node::MemPoolCheck;
use chain_impl_mockchain::fragment::FragmentId;
use jormungandr_lib::interfaces::FragmentStatus;
use std::time::Duration;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum FragmentVerifierError {
    #[error("fragment sent to node: {alias} is not in block :({status:?}). logs: {logs}")]
    FragmentNotInBlock {
        alias: String,
        status: FragmentStatus,
        logs: String,
    },
    #[error("transaction already balanced")]
    FragmentIsPendingForTooLong {
        fragment_id: FragmentId,
        timeout: Duration,
        alias: String,
        logs: String,
    },
    #[error(
        "fragment sent to node: {alias} is not in in fragment pool :({fragment_id}). logs: {logs}"
    )]
    FragmentNoInMemPoolLogs {
        alias: String,
        fragment_id: FragmentId,
        logs: String,
    },
    #[error("fragment verifier error")]
    FragmentVerifierError(#[from] FragmentNodeError),
}

pub struct FragmentVerifier;

impl FragmentVerifier {
    pub fn wait_and_verify_is_in_block<A: FragmentNode + ?Sized>(
        &self,
        duration: Duration,
        check: MemPoolCheck,
        node: &A,
    ) -> Result<(), FragmentVerifierError> {
        let status = self.wait_fragment(duration, check, node)?;
        self.is_in_block(status, node)
    }

    pub fn is_in_block<A: FragmentNode + ?Sized>(
        &self,
        status: FragmentStatus,
        node: &A,
    ) -> Result<(), FragmentVerifierError> {
        if !status.is_in_a_block() {
            return Err(FragmentVerifierError::FragmentNotInBlock {
                alias: node.alias().to_string(),
                status: status,
                logs: node.log_content(),
            });
        }
        Ok(())
    }

    pub fn fragment_status<A: FragmentNode + ?Sized>(
        &self,
        check: MemPoolCheck,
        node: &A,
    ) -> Result<FragmentStatus, FragmentVerifierError> {
        let logs = node.fragment_logs()?;
        if let Some(log) = logs.get(check.fragment_id()) {
            let status = log.status().clone();
            match log.status() {
                FragmentStatus::Pending => {
                    node.log_pending_fragment(check.fragment_id().clone());
                }
                FragmentStatus::Rejected { reason } => {
                    node.log_rejected_fragment(check.fragment_id().clone(), reason.to_string());
                }
                FragmentStatus::InABlock { date, block } => {
                    node.log_in_block_fragment(
                        check.fragment_id().clone(),
                        date.clone(),
                        block.clone(),
                    );
                }
            }
            return Ok(status);
        }

        Err(FragmentVerifierError::FragmentNoInMemPoolLogs {
            alias: node.alias().to_string(),
            fragment_id: check.fragment_id().clone(),
            logs: node.log_content(),
        })
    }

    pub fn wait_fragment<A: FragmentNode + ?Sized>(
        &self,
        duration: Duration,
        check: MemPoolCheck,
        node: &A,
    ) -> Result<FragmentStatus, FragmentVerifierError> {
        let max_try = 50;
        for _ in 0..max_try {
            let status_result = self.fragment_status(check.clone(), node);

            if let Err(_) = status_result {
                std::thread::sleep(duration);
                continue;
            }

            let status = status_result.unwrap();

            match status {
                FragmentStatus::Rejected { .. } => return Ok(status),
                FragmentStatus::InABlock { .. } => return Ok(status),
                _ => (),
            }
            std::thread::sleep(duration);
        }

        Err(FragmentVerifierError::FragmentIsPendingForTooLong {
            fragment_id: check.fragment_id().clone(),
            timeout: Duration::from_secs(duration.as_secs() * max_try),
            alias: node.alias().to_string(),
            logs: node.log_content(),
        })
    }
}
