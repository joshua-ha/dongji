//! UI와 엔진 사이의 계약(contract).
//! 엔진은 입력을 받을 때마다 `Turn` = (새 로그 목록 + 다음 프롬프트)를 돌려준다.
//! UI는 로그를 kind에 따라 그리고, 프롬프트의 선택지를 버튼으로 보여주면 된다.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LogKind {
    /// 장 제목, 엔딩 제목 등 큰 헤더
    Title,
    /// 지문
    Narration,
    /// 대사 (speaker 필드 사용)
    Dialogue,
    /// 안내/시스템 메시지
    System,
    /// HUD 성격의 상태 표시 (HP, 주력 등)
    Status,
    /// 일반 전투 로그
    Battle,
    /// 위험/피해/경고
    Danger,
    /// 보상/성장
    Reward,
}

impl LogKind {
    pub fn as_str(self) -> &'static str {
        match self {
            LogKind::Title => "title",
            LogKind::Narration => "narration",
            LogKind::Dialogue => "dialogue",
            LogKind::System => "system",
            LogKind::Status => "status",
            LogKind::Battle => "battle",
            LogKind::Danger => "danger",
            LogKind::Reward => "reward",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct LogEntry {
    pub kind: LogKind,
    pub speaker: Option<String>,
    pub text: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PromptKind {
    /// options 중 하나를 고른다 (0부터 시작하는 인덱스)
    Choice,
    /// 자유 텍스트 입력 (이름 입력 등)
    Text,
}

#[derive(Clone, Debug)]
pub struct Prompt {
    pub kind: PromptKind,
    pub title: String,
    pub options: Vec<String>,
    /// options와 같은 길이. UI가 아이콘/버튼 종류를 고르는 데 쓰는 안정적인 ID (예: "attack", "spot_0")
    pub ids: Vec<String>,
    /// options와 같은 길이. false면 지금은 쓸 수 없음 (예: 주력 부족)
    pub enabled: Vec<bool>,
}

#[derive(Clone, Debug)]
pub struct Turn {
    pub logs: Vec<LogEntry>,
    pub prompt: Prompt,
    /// 새 체크포인트가 생겼을 때만 Some. 호스트는 이 문자열을 그대로 저장해 두면 된다.
    pub checkpoint: Option<String>,
    /// 현재 화면 상태(JSON 객체 문자열). UI가 HP바/적 정보/장면 전환을 그릴 때 쓴다.
    pub state: String,
}

#[derive(Clone, Debug)]
pub enum Input {
    Choice(usize),
    Text(String),
}

pub(crate) fn json_escape(s: &str) -> String {
    let mut o = String::with_capacity(s.len() + 2);
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            '\r' => o.push_str("\\r"),
            '\t' => o.push_str("\\t"),
            c if (c as u32) < 0x20 => o.push_str(&format!("\\u{:04x}", c as u32)),
            c => o.push(c),
        }
    }
    o
}

impl LogEntry {
    pub fn to_json(&self) -> String {
        let speaker = match &self.speaker {
            Some(s) => format!("\"{}\"", json_escape(s)),
            None => "null".to_string(),
        };
        format!(
            "{{\"kind\":\"{}\",\"speaker\":{},\"text\":\"{}\"}}",
            self.kind.as_str(),
            speaker,
            json_escape(&self.text)
        )
    }
}

impl Turn {
    /// iOS(Swift)에서 `JSONDecoder`로 그대로 디코딩할 수 있는 형태.
    /// {"logs":[...],"prompt":{"type":"choice|text","title","options":[...]},"checkpoint":string|null}
    pub fn to_json(&self) -> String {
        let logs: Vec<String> = self.logs.iter().map(|l| l.to_json()).collect();
        let opts: Vec<String> = self
            .prompt
            .options
            .iter()
            .map(|o| format!("\"{}\"", json_escape(o)))
            .collect();
        let ids: Vec<String> = self.prompt.ids.iter().map(|o| format!("\"{}\"", json_escape(o))).collect();
        let enabled: Vec<&str> = self.prompt.enabled.iter().map(|b| if *b { "true" } else { "false" }).collect();
        let kind = match self.prompt.kind {
            PromptKind::Choice => "choice",
            PromptKind::Text => "text",
        };
        let checkpoint = match &self.checkpoint {
            Some(c) => format!("\"{}\"", json_escape(c)),
            None => "null".to_string(),
        };
        format!(
            "{{\"logs\":[{}],\"prompt\":{{\"type\":\"{}\",\"title\":\"{}\",\"options\":[{}],\"ids\":[{}],\"enabled\":[{}]}},\"checkpoint\":{},\"state\":{}}}",
            logs.join(","),
            kind,
            json_escape(&self.prompt.title),
            opts.join(","),
            ids.join(","),
            enabled.join(","),
            checkpoint,
            self.state
        )
    }
}
