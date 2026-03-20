/// Prompt templates for AI-powered analysis

pub fn security_analysis_prompt(context: &str) -> String {
    format!(
        "You are a security auditor. Analyze the following code findings for OWASP Top 10 \
         vulnerabilities and security risks. For each issue, provide:\n\
         1. Risk severity (Critical/High/Medium/Low)\n\
         2. Specific vulnerability type\n\
         3. Recommended fix\n\n\
         Findings:\n{}\n\n\
         Provide a concise JSON response with: summary, risk_score (0-100), and recommendations array.",
        context
    )
}

pub fn api_efficiency_prompt(context: &str) -> String {
    format!(
        "You are an API performance expert. Analyze these API endpoints and functions for:\n\
         1. N+1 query patterns\n\
         2. Missing pagination on list endpoints\n\
         3. Redundant API calls\n\
         4. Missing rate limiting\n\
         5. Inefficient data fetching patterns\n\n\
         Findings:\n{}\n\n\
         Provide a concise JSON response with: summary, risk_score (0-100), and recommendations array.",
        context
    )
}

pub fn agent_data_prompt(context: &str) -> String {
    format!(
        "You are an AI safety researcher. Analyze these agent configurations for:\n\
         1. Overly broad data access permissions\n\
         2. Potential data exfiltration paths\n\
         3. Unscoped tool access\n\
         4. Missing sandboxing for code execution\n\
         5. Token/credential exposure to agents\n\n\
         Findings:\n{}\n\n\
         Provide a concise JSON response with: summary, risk_score (0-100), and recommendations array.",
        context
    )
}
