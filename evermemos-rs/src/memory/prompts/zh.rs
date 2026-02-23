/// Chinese prompts for all memory extraction stages.
/// These mirror the en module but with Chinese prompt text,
/// for users who configured the zh locale.

pub const BOUNDARY_DETECTION_SYSTEM: &str = r#"
你是一个对话边界检测器。你的任务是判断对话是否已到达自然话题边界，需要保存为记忆单元。

当以下情况发生时，应触发边界：
1. 对话话题明显转变为无关内容
2. 达成了重要结论或事件
3. 积累了足够多的有意义信息（通常5轮以上）
4. 对话出现自然停顿或结束

返回JSON对象：
{
  "is_boundary": boolean,
  "reason": "简短说明"
}
"#;

pub const BOUNDARY_DETECTION_USER: &str = r#"
历史对话（已保存为之前的记忆）：
{history}

新对话消息：
{new_messages}

判断新消息与历史对话合并后是否构成完整的记忆单元，应该现在保存。
"#;

pub const EPISODE_GENERATION_SYSTEM: &str = r#"
你是一个个人记忆提取器。从对话中提取连贯的情节记忆。

创建第一人称叙事记忆，包含：
- 发生了什么或讨论了什么
- 重要细节、事实和决定
- 相关的情感背景
- 提到的时间标记

返回JSON对象：
{
  "summary": "一行简洁摘要（最多100字）",
  "episode": "第一人称完整叙述，2-5句话",
  "subject": "本次记忆的主题",
  "keywords": ["关键词1", "关键词2"],
  "participants": ["人物1", "人物2"]
}
"#;

pub const EPISODE_GENERATION_USER: &str =
    crate::memory::prompts::en::EPISODE_GENERATION_USER;

pub const GROUP_EPISODE_GENERATION_SYSTEM: &str = r#"
你是一个群组对话记忆提取器。从群组对话中提取共同的情节记忆。

创建第三人称叙述，包含：
- 群组讨论或决定的内容
- 不同参与者的主要观点
- 结果、协议或行动项目
- 重要的共享信息

返回JSON对象：
{
  "summary": "一行简洁摘要（最多100字）",
  "episode": "第三人称完整叙述，2-5句话",
  "subject": "主题",
  "keywords": ["关键词1", "关键词2"],
  "participants": ["人物1", "人物2"]
}
"#;

pub const GROUP_EPISODE_GENERATION_USER: &str =
    crate::memory::prompts::en::GROUP_EPISODE_GENERATION_USER;

pub const FORESIGHT_GENERATION_SYSTEM: &str = r#"
你是一个预见性记忆提取器。分析对话，识别用户提到的未来事件、计划、意图或预测。

寻找：
- 已安排的事件或约定
- 明确的意图或计划（"我会"、"我打算"、"我计划"）
- 预测或期望
- 截止日期或时间绑定的承诺

返回JSON数组（如无预见则返回空数组）：
[
  {
    "foresight": "预测/计划事件的描述",
    "evidence": "对话中支持此预见的直接引用",
    "start_time": "ISO8601日期时间或null",
    "end_time": "ISO8601日期时间或null",
    "duration_days": 整数或null
  }
]
"#;

pub const FORESIGHT_GENERATION_USER: &str =
    crate::memory::prompts::en::FORESIGHT_GENERATION_USER;

pub const EVENT_LOG_SYSTEM: &str = r#"
你是一个原子事实提取器。从对话中提取离散的、可验证的事实。

每条事实应该：
- 是单一的、独立的陈述
- 具体明确（不含糊）
- 关于用户、其生活、偏好或经历
- 写成完整的句子

返回JSON数组（如无事实则返回空数组）：
[
  {"atomic_fact": "用户在TechCorp担任软件工程师。"},
  {"atomic_fact": "用户在后端开发中更偏好Python而非JavaScript。"}
]
"#;

pub const EVENT_LOG_USER: &str = crate::memory::prompts::en::EVENT_LOG_USER;
pub const PROFILE_PART1_SYSTEM: &str = crate::memory::prompts::en::PROFILE_PART1_SYSTEM;
pub const PROFILE_PART1_USER: &str = crate::memory::prompts::en::PROFILE_PART1_USER;
pub const PROFILE_PART2_SYSTEM: &str = crate::memory::prompts::en::PROFILE_PART2_SYSTEM;
pub const PROFILE_PART2_USER: &str = crate::memory::prompts::en::PROFILE_PART2_USER;
pub const PROFILE_LIFE_UPDATE_SYSTEM: &str = crate::memory::prompts::en::PROFILE_LIFE_UPDATE_SYSTEM;
pub const PROFILE_LIFE_UPDATE_USER: &str = crate::memory::prompts::en::PROFILE_LIFE_UPDATE_USER;
pub const PROFILE_LIFE_INITIAL_SYSTEM: &str = crate::memory::prompts::en::PROFILE_LIFE_INITIAL_SYSTEM;
pub const PROFILE_LIFE_INITIAL_USER: &str = crate::memory::prompts::en::PROFILE_LIFE_INITIAL_USER;
