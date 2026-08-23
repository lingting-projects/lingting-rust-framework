use anyhow::{Result, anyhow};
use framework_datetime::current_millis;
use std::sync::{LazyLock, Mutex};

const DEFAULT_EPOCH_MILLIS: i64 = 1735689600000;
const SEQUENCE_MASK: u16 = 0x0fff;

struct SnowflakeState {
    last_millis: i64,
    sequence: u16,
}

static GLOBAL_SNOWFLAKE: LazyLock<Snowflake> = LazyLock::new(|| Snowflake::new(1, 1));

pub struct Snowflake {
    epoch_millis: i64,
    node: u16,
    state: Mutex<SnowflakeState>,
}

impl Snowflake {
    pub fn new(datacenter: u8, worker: u8) -> Self {
        Self::new_with_epoch(datacenter, worker, DEFAULT_EPOCH_MILLIS)
    }

    pub fn new_with_epoch(datacenter: u8, worker: u8, epoch_millis: i64) -> Self {
        Self {
            epoch_millis,
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
            .checked_sub(self.epoch_millis)
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
