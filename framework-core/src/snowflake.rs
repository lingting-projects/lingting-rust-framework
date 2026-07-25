use anyhow::{Result, anyhow};
use framework_datetime::current_millis;
use std::sync::{LazyLock, Mutex};

const EPOCH: u64 = 1_704_067_200_000;
const SEQUENCE_MASK: u16 = 0x0fff;

struct SnowflakeState {
    last_millis: u64,
    sequence: u16,
}

static GLOBAL_SNOWFLAKE: LazyLock<Snowflake> = LazyLock::new(|| Snowflake::new(1, 1));

pub struct Snowflake {
    node: u16,
    state: Mutex<SnowflakeState>,
}

impl Snowflake {
    pub fn new(datacenter: u8, worker: u8) -> Self {
        Self {
            node: (u16::from(datacenter & 0x1f) << 5) | u16::from(worker & 0x1f),
            state: Mutex::new(SnowflakeState {
                last_millis: 0,
                sequence: 0,
            }),
        }
    }

    pub fn next_id(&self) -> Result<u64> {
        let mut state = self
            .state
            .lock()
            .map_err(|error| anyhow!("雪花 ID 状态锁定失败: {error}"))?;
        let current_millis = current_millis()?;
        let mut logical_millis = current_millis.max(state.last_millis);

        if logical_millis == state.last_millis {
            state.sequence = (state.sequence + 1) & SEQUENCE_MASK;
            if state.sequence == 0 {
                logical_millis = state.last_millis.saturating_add(1);
            }
        } else {
            state.sequence = 0;
        }

        state.last_millis = logical_millis;
        Ok((logical_millis.saturating_sub(EPOCH) << 22)
            | (u64::from(self.node) << 12)
            | u64::from(state.sequence))
    }
}

pub fn next_id() -> Result<u64> {
    GLOBAL_SNOWFLAKE.next_id()
}
