//! Minimal deterministic command gate: no networking, credentials or persistence.
//! Future command adapters must share this fail-closed core entrypoint.

use crate::sdp::{inspect_sdp, SdpError, SdpSummary};

pub const MAX_COMMAND_INPUT: usize = 64 * 1024;
pub const INSPECT_SDP_ID: &str = "diagnostics.inspect-sdp.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandError {
    UnknownCommand,
    ConsentRequired,
    PolicyDenied,
    InvalidInput,
    InputTooLarge,
    MalformedSdp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommandContext {
    pub user_consented: bool,
    pub policy_allows: bool,
}

pub fn execute_offline(
    command_id: &str,
    bytes: &[u8],
    context: CommandContext,
) -> Result<SdpSummary, CommandError> {
    // Deny unknown operations before authorization or parsing. There is
    // intentionally no network-capable command, fallback or token API.
    if command_id != INSPECT_SDP_ID {
        return Err(CommandError::UnknownCommand);
    }
    if !context.user_consented {
        return Err(CommandError::ConsentRequired);
    }
    if !context.policy_allows {
        return Err(CommandError::PolicyDenied);
    }
    if bytes.len() > MAX_COMMAND_INPUT {
        return Err(CommandError::InputTooLarge);
    }
    if bytes.is_empty() {
        return Err(CommandError::InvalidInput);
    }
    inspect_sdp(bytes).map_err(|err| match err {
        SdpError::TooLarge => CommandError::InputTooLarge,
        _ => CommandError::MalformedSdp,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &[u8] =
        b"v=0\r\no=- 1 1 IN IP4 192.0.2.1\r\ns=-\r\nt=0 0\r\nm=audio 49170 RTP/AVP 0\r\n";

    fn allowed() -> CommandContext {
        CommandContext {
            user_consented: true,
            policy_allows: true,
        }
    }

    #[test]
    fn deterministic_read_only_diagnostic() {
        let first = execute_offline(INSPECT_SDP_ID, SAMPLE, allowed()).unwrap();
        let second = execute_offline(INSPECT_SDP_ID, SAMPLE, allowed()).unwrap();
        assert_eq!(first, second);
        assert_eq!(first.media.len(), 1);
    }

    #[test]
    fn deny_unknown_or_network_commands() {
        for id in [
            "",
            "sip.register",
            "presence.publish",
            "diagnostics.inspect-sdp.v2",
        ] {
            assert_eq!(
                execute_offline(id, SAMPLE, allowed()),
                Err(CommandError::UnknownCommand)
            );
        }
    }

    #[test]
    fn consent_and_policy_are_both_mandatory() {
        for (context, expected) in [
            (
                CommandContext {
                    user_consented: false,
                    policy_allows: true,
                },
                CommandError::ConsentRequired,
            ),
            (
                CommandContext {
                    user_consented: true,
                    policy_allows: false,
                },
                CommandError::PolicyDenied,
            ),
            (
                CommandContext {
                    user_consented: false,
                    policy_allows: false,
                },
                CommandError::ConsentRequired,
            ),
        ] {
            assert_eq!(
                execute_offline(INSPECT_SDP_ID, SAMPLE, context),
                Err(expected)
            );
        }
    }

    #[test]
    fn malformed_empty_and_oversized_inputs_fail_closed() {
        assert_eq!(
            execute_offline(INSPECT_SDP_ID, b"", allowed()),
            Err(CommandError::InvalidInput)
        );
        assert_eq!(
            execute_offline(INSPECT_SDP_ID, b"v=0\n", allowed()),
            Err(CommandError::MalformedSdp)
        );
        let huge = vec![b'a'; MAX_COMMAND_INPUT + 1];
        assert_eq!(
            execute_offline(INSPECT_SDP_ID, &huge, allowed()),
            Err(CommandError::InputTooLarge)
        );
    }
}
