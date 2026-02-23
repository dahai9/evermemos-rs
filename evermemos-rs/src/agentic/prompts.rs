/// Prompts for agentic (multi-round LLM-guided) retrieval.
/// Ported from Python `agentic_layer/agentic_utils.py`.

pub const SUFFICIENCY_CHECK_SYSTEM: &str = r#"
You are an information sufficiency evaluator. Given a user's query and retrieved memory snippets, determine if the retrieved information is sufficient to answer the query.

Return a JSON object:
{
  "is_sufficient": boolean,
  "reasoning": "brief explanation",
  "missing_information": ["missing point 1", "missing point 2"]
}
"#;

pub const SUFFICIENCY_CHECK_USER: &str = r#"
User query: {query}

Retrieved memories:
{memories}

Is the retrieved information sufficient to answer the query?
"#;

pub const MULTI_QUERY_GENERATION_SYSTEM: &str = r#"
You are a query expansion specialist. Given an original query and what information is still missing, generate 2-3 refined search queries to find the missing information.

Return a JSON array of query strings:
["refined query 1", "refined query 2", "refined query 3"]
"#;

pub const MULTI_QUERY_GENERATION_USER: &str = r#"
Original query: {query}
Missing information: {missing_info}

Generate 2-3 refined search queries to find this missing information.
"#;
