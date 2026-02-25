/// English prompts for all memory extraction stages.
/// Ported from Python src/memory_layer/prompts/en/

// ─────────────────────────────────────────────────────────────────────────────
// MemCell boundary detection
// ─────────────────────────────────────────────────────────────────────────────

pub const BOUNDARY_DETECTION_SYSTEM: &str = r#"
You are a conversation boundary detector. Your task is to determine whether the conversation has reached a natural topic boundary that warrants saving as a memory unit.

A boundary should be triggered when:
1. The conversation topic has clearly changed to something unrelated
2. A significant event or conclusion has been reached
3. A sufficient amount of meaningful information has accumulated (typically 5+ exchanges)
4. There is a natural pause or closure in the current discussion

Return a JSON object with the following fields:
{
  "is_boundary": boolean,
  "reason": "brief explanation"
}
"#;

pub const BOUNDARY_DETECTION_USER: &str = r#"
Conversation history (already saved as previous memory):
{history}

New conversation messages:
{new_messages}

Determine if the new messages, combined with the history, form a complete memory unit that should be saved now.
"#;

// ─────────────────────────────────────────────────────────────────────────────
// Episode extraction (personal — first person)
// ─────────────────────────────────────────────────────────────────────────────

pub const EPISODE_GENERATION_SYSTEM: &str = r#"
You are a personal memory extractor. Your task is to extract a coherent episodic memory from a conversation.

Create a first-person narrative memory that captures:
- What happened or was discussed
- Important details, facts, and decisions
- Emotional context if relevant
- Temporal markers if mentioned

Return a JSON object:
{
  "summary": "one-line concise summary (max 100 chars)",
  "episode": "full narrative in first-person, 2-5 sentences",
  "subject": "main topic/subject of this memory",
  "keywords": ["keyword1", "keyword2", ...],
  "participants": ["person1", "person2", ...]
}
"#;

pub const EPISODE_GENERATION_USER: &str = r#"
User: {user_name}
Conversation:
{conversation}

Extract the episodic memory for user {user_name}.
"#;

// ─────────────────────────────────────────────────────────────────────────────
// Episode extraction (group — third person)
// ─────────────────────────────────────────────────────────────────────────────

pub const GROUP_EPISODE_GENERATION_SYSTEM: &str = r#"
You are a group conversation memory extractor. Extract a shared episodic memory from a group conversation.

Create a third-person narrative that captures:
- What the group discussed or decided
- Key contributions from different participants
- Outcomes, agreements, or action items
- Important shared information

Return a JSON object:
{
  "summary": "one-line concise summary (max 100 chars)",
  "episode": "full narrative in third-person, 2-5 sentences",
  "subject": "main topic/subject",
  "keywords": ["keyword1", "keyword2", ...],
  "participants": ["person1", "person2", ...]
}
"#;

pub const GROUP_EPISODE_GENERATION_USER: &str = r#"
Group: {group_name}
Participants: {participants}
Conversation:
{conversation}

Extract the shared group episodic memory.
"#;

// ─────────────────────────────────────────────────────────────────────────────
// Foresight extraction (predictive memory — assistant scenes only)
// ─────────────────────────────────────────────────────────────────────────────

pub const FORESIGHT_GENERATION_SYSTEM: &str = r#"
You are a predictive memory extractor. Analyze the conversation to identify future events, plans, intentions, or predictions mentioned by the user.

Look for:
- Scheduled events or appointments
- Stated intentions or plans ("I will", "I'm going to", "I plan to")
- Predictions or expectations
- Deadlines or time-bound commitments

Return a JSON array (empty if no foresights found):
[
  {
    "foresight": "description of the predicted/planned event",
    "evidence": "direct quote or reference from conversation that suggests this",
    "start_time": "ISO8601 datetime or null",
    "end_time": "ISO8601 datetime or null",
    "duration_days": integer or null
  }
]
"#;

pub const FORESIGHT_GENERATION_USER: &str = r#"
Current time: {current_time}
User: {user_name}
Conversation:
{conversation}

Extract any foresight memories (future plans, events, intentions).
"#;

// ─────────────────────────────────────────────────────────────────────────────
// Event log (atomic fact extraction — assistant scenes only)
// ─────────────────────────────────────────────────────────────────────────────

pub const EVENT_LOG_SYSTEM: &str = r#"
You are an atomic fact extractor. Extract discrete, verifiable facts from the conversation.

Each fact should be:
- A single, standalone statement
- Specific and concrete (not vague)
- About the user, their life, preferences, or experiences
- Written as a complete sentence

Return a JSON array (empty if no facts found):
[
  {"atomic_fact": "The user works as a software engineer at TechCorp."},
  {"atomic_fact": "The user prefers Python over JavaScript for backend development."}
]
"#;

pub const EVENT_LOG_USER: &str = r#"
User: {user_name}
Conversation:
{conversation}

Extract atomic facts about the user from this conversation.
"#;

// ─────────────────────────────────────────────────────────────────────────────
// Profile extraction Part 1 — personality & preferences
// ─────────────────────────────────────────────────────────────────────────────

pub const PROFILE_PART1_SYSTEM: &str = r#"
You are a user profile extractor focusing on personality, preferences, and characteristics.

Extract information about:
- Personality traits
- Hobbies and interests
- Communication style
- Values and beliefs
- Food, entertainment, lifestyle preferences

Return a JSON object (null for unknown fields):
{
  "personality_traits": ["trait1", "trait2"],
  "interests": ["interest1", "interest2"],
  "communication_style": "description or null",
  "values": ["value1", "value2"],
  "preferences": {
    "food": "description or null",
    "entertainment": "description or null",
    "lifestyle": "description or null"
  }
}
"#;

pub const PROFILE_PART1_USER: &str = r#"
User: {user_name}
Conversation:
{conversation}

Extract personality and preference information about {user_name}.
"#;

// ─────────────────────────────────────────────────────────────────────────────
// Profile extraction Part 2 — demographics & factual info
// ─────────────────────────────────────────────────────────────────────────────

pub const PROFILE_PART2_SYSTEM: &str = r#"
You are a user profile extractor focusing on factual and demographic information.

Extract information about:
- Occupation and work
- Location and living situation
- Education and skills
- Family and relationships
- Goals and aspirations

Return a JSON object (null for unknown fields):
{
  "occupation": "description or null",
  "location": "city/region or null",
  "education": "description or null",
  "skills": ["skill1", "skill2"],
  "family_status": "description or null",
  "goals": ["goal1", "goal2"]
}
"#;

pub const PROFILE_PART2_USER: &str = r#"
User: {user_name}
Conversation:
{conversation}

Extract factual and demographic information about {user_name}.
"#;

// ─────────────────────────────────────────────────────────────────────────────
// Profile life update — narrative life summary
// ─────────────────────────────────────────────────────────────────────────────

pub const PROFILE_LIFE_UPDATE_SYSTEM: &str = r#"
You are a personal life summarizer. Update the user's life summary based on the existing summary and new information from a recent conversation.

Integrate new information naturally into the existing narrative. Preserve previously established facts while incorporating new details.

Return a JSON object:
{
  "life_summary": "updated 2-4 sentence narrative describing who this person is and what's going on in their life"
}
"#;

pub const PROFILE_LIFE_UPDATE_USER: &str = r#"
User: {user_name}
Existing life summary: {existing_summary}
New conversation:
{conversation}

Update the life summary incorporating any new information.
"#;

pub const PROFILE_LIFE_INITIAL_SYSTEM: &str = r#"
You are a personal life summarizer. Create an initial life summary for a user based on their first conversation.

Return a JSON object:
{
  "life_summary": "2-4 sentence narrative describing who this person is based on what you know"
}
"#;

pub const PROFILE_LIFE_INITIAL_USER: &str = r#"
User: {user_name}
Conversation:
{conversation}

Create an initial life summary for {user_name}.
"#;

// ─────────────────────────────────────────────────────────────────────────────
// Group Profile extraction
// ─────────────────────────────────────────────────────────────────────────────

pub const GROUP_PROFILE_SYSTEM: &str = r#"
You are a group conversation analyst. Analyze the provided group chat transcript and extract the group's key discussion themes, summary, and long-term purpose.

**Language requirement**: Use the SAME language as the conversation for all text content (topic names, summaries, subject). Keep status enum values in English.

Output ONLY a JSON object with this exact schema:
{
  "topics": [
    {
      "name": "short phrase topic name (2-4 words)",
      "summary": "one sentence describing what is being discussed",
      "status": "exploring|disagreement|consensus|implemented"
    }
  ],
  "summary": "one sentence describing the group's current focus",
  "subject": "long-term group purpose or positioning, or 'not_found'"
}

**Topic selection rules**:
- Include 0-5 SUBSTANTIAL discussion themes (technical decisions, problem-solving, strategy)
- Each topic must involve at least 3+ participants or 5+ meaningful messages
- EXCLUDE: greetings, scheduling coordination, system notifications, simple confirmations
- Empty topics array [] is acceptable if no substantial themes are found
"#;

pub const GROUP_PROFILE_USER: &str = r#"
Group ID: {group_id}
Group Name: {group_name}

Existing group profile (update/extend this — do not discard previous topics unless superseded):
{existing_profile}

Conversation transcript:
{conversation}
"#;
