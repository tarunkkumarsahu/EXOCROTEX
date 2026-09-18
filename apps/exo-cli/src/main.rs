use std::io::{self, Write};

use cognitive_core::{
    ActionRisk, ApprovalState, CognitiveKernel, EpistemicType, MemoryKind,
};
use serde_json::json;
use uuid::Uuid;

fn main() {
    let mut kernel = CognitiveKernel::new();

    println!("EXOCORTEX Cognitive Kernel v0.1");
    println!("Autonomous cognition, permissioned action.");
    println!("Type :help for commands.\n");

    loop {
        print!("exo> ");
        if io::stdout().flush().is_err() {
            break;
        }

        let mut input = String::new();
        match io::stdin().read_line(&mut input) {
            Ok(0) => break,
            Ok(_) => {}
            Err(error) => {
                eprintln!("input error: {error}");
                break;
            }
        }

        let input = input.trim();
        if input.is_empty() {
            continue;
        }

        if matches!(input, ":exit" | ":quit") {
            break;
        }

        if input == ":help" {
            print_help();
            continue;
        }

        if input == ":status" {
            println!(
                "events={} memories={} pending_approvals={}",
                kernel.event_count(),
                kernel.memory_count(),
                kernel.pending_actions().len()
            );
            continue;
        }

        if let Some(text) = input.strip_prefix(":remember ") {
            let event = kernel.observe_user(text);
            let memory = kernel.remember(
                text,
                MemoryKind::Semantic,
                EpistemicType::UserConfirmedFact,
                event.id,
            );

            println!(
                "remembered {} as {:?} ({:?})",
                memory.id, memory.kind, memory.epistemic_type
            );
            continue;
        }

        if let Some(query) = input.strip_prefix(":recall ") {
            let memories = kernel.recall(query);

            if memories.is_empty() {
                println!("no matching memories");
            } else {
                for memory in memories {
                    println!(
                        "{} | {:?} | {:?} | {}",
                        memory.id, memory.kind, memory.epistemic_type, memory.text
                    );
                }
            }
            continue;
        }

        if let Some(summary) = input.strip_prefix(":propose ") {
            let proposal = kernel.propose_action(
                "manual_external_action",
                summary,
                ActionRisk::ExternalWrite,
                json!({"summary": summary}),
            );

            println!(
                "proposal {} | risk={:?} | approval={:?}",
                proposal.id, proposal.risk, proposal.approval_state
            );

            if proposal.approval_state == ApprovalState::Pending {
                println!("human approval required before execution");
            }
            continue;
        }

        if input == ":pending" {
            let pending = kernel.pending_actions();

            if pending.is_empty() {
                println!("no pending approvals");
            } else {
                for action in pending {
                    println!("{} | {:?} | {}", action.id, action.risk, action.summary);
                }
            }
            continue;
        }

        if let Some(raw_id) = input.strip_prefix(":approve ") {
            handle_approval(&mut kernel, raw_id, true);
            continue;
        }

        if let Some(raw_id) = input.strip_prefix(":reject ") {
            handle_approval(&mut kernel, raw_id, false);
            continue;
        }

        let event = kernel.observe_user(input);
        println!(
            "captured user event {}. Use :remember <text> to persist a confirmed memory.",
            event.id
        );
    }
}

fn handle_approval(kernel: &mut CognitiveKernel, raw_id: &str, approve: bool) {
    let id = match Uuid::parse_str(raw_id.trim()) {
        Ok(id) => id,
        Err(error) => {
            println!("invalid UUID: {error}");
            return;
        }
    };

    let result = if approve {
        kernel.approve_action(id)
    } else {
        kernel.reject_action(id)
    };

    match result {
        Some(action) => println!("{} -> {:?}", action.id, action.approval_state),
        None => println!("proposal not found"),
    }
}

fn print_help() {
    println!(
        r#"Commands:
  :remember <text>   store a user-confirmed semantic memory
  :recall <query>    recall matching memories
  :propose <summary> create an external-write action proposal
  :pending           list actions waiting for human approval
  :approve <uuid>    approve a pending action
  :reject <uuid>     reject a pending action
  :status            show kernel counts
  :help              show this help
  :exit              quit
"#
    );
}
