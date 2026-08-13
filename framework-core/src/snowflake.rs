use anyhow::{Result, anyhow};
use framework_datetime::current_millis;
use std::sync::{LazyLock, Mutex};

const EPOCH: i64 = 1_704_067_200_000;
const SEQUENCE_MASK: u16 = 0x0fff;

struct SnowflakeState {
    last_millis: i64,
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

    pub fn next_id(&self) -> Result<i64> {
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
        let elapsed_millis = logical_millis
            .checked_sub(EPOCH)
            .ok_or_else(|| anyhow!("雪花时间差超出 i64 范围"))?;
        let timestamp_part = elapsed_millis
            .checked_mul(1 << 22)
            .ok_or_else(|| anyhow!("雪花 ID 超出 i64 范围"))?;
        let node_and_sequence = (i64::from(self.node) << 12) | i64::from(state.sequence);
        timestamp_part
            .checked_add(node_and_sequence)
            .ok_or_else(|| anyhow!("雪花 ID 超出 i64 范围"))
    }
}

pub fn next_id() -> Result<i64> {
    GLOBAL_SNOWFLAKE.next_id()
}
