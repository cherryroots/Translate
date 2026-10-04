use crate::config::Config;
use serde_json::{json, Value};
use std::time::Duration;

const TO_ENGLISH: &str = "You translate World of Warcraft in-game chat from Chinese into natural, casual English, the way an English-speaking player would write it. \
Keep player names, numbers, item names in brackets, and English words as they are. In chat, 1 usually means yes/ready. \
Reply with the translation only: no quotes, notes or romanization.";

const TO_CHINESE: &str = "Translate the user's World of Warcraft chat message into casual Simplified Chinese, the way a Chinese player would type it in game chat. \
Keep player names, numbers and item names as they are. Reply with the translation only.";

#[derive(Clone, Copy)]
pub enum Direction {
    ToEnglish,
    ToChinese,
}

/// Chinese WoW slang (Classic and retail) as `zh=en` pairs. Only the terms
/// found in a line are sent with it, so a long list costs no extra tokens.
const SLANG: &str = "\
奶妈=healer;奶=healer;治疗=healer;坦克=tank;缺T=need a tank;缺奶=need a healer;输出=DPS;近战=melee;远程=ranged;\
战士=warrior;防战=prot warrior;狂暴战=fury warrior;武器战=arms warrior;圣骑=paladin;奶骑=holy paladin;防骑=prot paladin;惩戒骑=ret paladin;\
猎人=hunter;兽王猎=BM hunter;射击猎=MM hunter;生存猎=survival hunter;盗贼=rogue;牧师=priest;奶牧=holy priest;戒律=disc priest;暗牧=shadow priest;\
萨满=shaman;奶萨=resto shaman;增强萨=enh shaman;元素萨=ele shaman;法师=mage;冰法=frost mage;火法=fire mage;奥法=arcane mage;\
术士=warlock;痛苦术=affli lock;毁灭术=destro lock;恶魔术=demo lock;德鲁伊=druid;小德=druid;奶德=resto druid;熊德=bear tank;猫德=feral druid;鸟德=boomkin;\
死骑=death knight;武僧=monk;恶魔猎手=demon hunter;唤魔师=evoker;\
组队=LFG;来人=LF more;缺人=LF more;满了=full;速来=come quick;集合=meet up;拉怪=pull;开怪=pull;别拉=don't pull;团灭=wipe;灭了=wiped;\
跑尸=corpse run;掉线=disconnected;进本=entering the dungeon;副本=dungeon;团本=raid;地下城=dungeon;大秘境=Mythic+;大米=Mythic+;钥石=keystone;\
英雄=heroic;史诗=mythic;老一=first boss;老二=second boss;尾王=last boss;小怪=trash;拉人=summon;召唤=summon;车队=group run;开车=starting the run;\
老板=buyer;打工=booster;带人=carry;求带=LF carry;代练=boosting;需求=need;贪婪=greed;分装=loot split;毕业=BiS;拍卖行=AH;收购=WTB;出售=WTS;\
公会=guild;工会=guild;收人=recruiting;招人=recruiting;散人=pug;大佬=pro;萌新=new player;菜鸟=noob;稍等=one sec;马上=coming;挂机=AFK;下线=logging off;\
战场=battleground;竞技场=arena;部落=Horde;联盟=Alliance;\
熔火之心=Molten Core;黑翼=BWL;祖格=ZG;安其拉=AQ;纳克萨玛斯=Naxx;死矿=Deadmines;血色=Scarlet Monastery;斯坦索姆=Stratholme;通灵学院=Scholomance;黑石深渊=BRD;厄运之槌=Dire Maul;\
任务=quest;做任务=questing;接任务=pick up the quest;交任务=turn in the quest;任务线=quest chain;任务路线=quest route;主线=main questline;支线=side quest;\
日常=dailies;周常=weeklies;声望=reputation;崇拜=exalted;崇敬=revered;练级=leveling;满级=max level;经验=XP;\
刷本=farming dungeons;打本=running dungeons;刷怪=grinding mobs;精英=elite;稀有=rare;世界首领=world boss;野外=open world;\
坐骑=mount;飞行点=flight path;炉石=hearthstone;传送门=portal;邮箱=mailbox;金币=gold;装等=item level;装备=gear;附魔=enchant;宝石=gem;\
专业=profession;采药=herbalism;挖矿=mining;剥皮=skinning;锻造=blacksmithing;炼金=alchemy;裁缝=tailoring;工程=engineering;珠宝=jewelcrafting;铭文=inscription;烹饪=cooking;钓鱼=fishing;急救=first aid;\
仇恨=aggro;嘲讽=taunt;打断=interrupt;驱散=dispel;变羊=sheep;复活=res;战复=battle res;嗜血=Bloodlust;没蓝=OOM;回蓝=drinking;喝水=drinking;\
邀请=invite;组我=invite me;密我=whisper me;退组=leaving the group;求组=LFG;路线=route";

/// The slang terms in a line, longest first, skipping any term that is
/// part of a longer match (奶 inside 奶骑). At most 12 to keep prompts short.
fn slang_in(text: &str) -> Vec<(&'static str, &'static str)> {
    let mut found: Vec<(&str, &str)> = SLANG
        .split(';')
        .filter_map(|pair| pair.split_once('='))
        .filter(|(zh, _)| text.contains(zh))
        .collect();
    found.sort_by_key(|(zh, _)| std::cmp::Reverse(zh.chars().count()));
    let mut kept: Vec<(&str, &str)> = Vec::new();
    for (zh, en) in found {
        if !kept.iter().any(|(longer, _)| longer.contains(zh)) {
            kept.push((zh, en));
        }
    }
    kept.truncate(12);
    kept
}

/// Tencent's HY-MT models want their own prompt and no system message.
fn is_hy_mt(model: &str) -> bool {
    let m = model.to_lowercase();
    m.contains("hy-mt") || m.contains("hunyuan-mt")
}

/// Builds the request body for either prompt style.
fn request_body(model: &str, text: &str, direction: Direction) -> Value {
    if !is_hy_mt(model) {
        let system = match direction {
            Direction::ToEnglish => {
                let slang: Vec<String> = slang_in(text).iter().map(|(zh, en)| format!("{zh} = {en}")).collect();
                if slang.is_empty() {
                    TO_ENGLISH.to_string()
                } else {
                    format!("{TO_ENGLISH} Slang in this line: {}.", slang.join(", "))
                }
            }
            Direction::ToChinese => TO_CHINESE.to_string(),
        };
        return json!({
            "model": model,
            "temperature": 0.2,
            "messages": [
                { "role": "system", "content": system },
                { "role": "user", "content": text },
            ],
        });
    }

    // Templates and sampling settings from the HY-MT1.5 model card.
    let target = match direction {
        Direction::ToEnglish => "英语",
        Direction::ToChinese => "中文",
    };
    let terms: Vec<String> = match direction {
        Direction::ToEnglish => slang_in(text)
            .iter()
            .map(|(zh, en)| format!("{zh} 翻译成 {en}"))
            .collect(),
        Direction::ToChinese => Vec::new(),
    };
    let prompt = if terms.is_empty() {
        format!("将以下文本翻译为{target}，注意只需要输出翻译后的结果，不要额外解释：\n\n{text}")
    } else {
        format!(
            "参考下面的翻译：\n{}\n\n将以下文本翻译为{target}，注意只需要输出翻译后的结果，不要额外解释：\n{text}",
            terms.join("\n")
        )
    };
    json!({
        "model": model,
        "temperature": 0.7,
        "top_p": 0.6,
        // Read by llama.cpp and Ollama; other servers ignore them.
        "top_k": 20,
        "repeat_penalty": 1.05,
        "messages": [{ "role": "user", "content": prompt }],
    })
}

/// Local servers (LM Studio, Ollama) serve the API under /v1; a bare
/// `http://host:port` gets it added so a missing /v1 is not a silent failure.
fn completions_url(base: &str) -> String {
    let base = base.trim().trim_end_matches('/');
    let after_scheme = base.split_once("://").map_or(base, |(_, rest)| rest);
    if after_scheme.contains('/') {
        format!("{base}/chat/completions")
    } else {
        format!("{base}/v1/chat/completions")
    }
}

/// Sends one message to an OpenAI-compatible /chat/completions endpoint.
pub fn translate(config: &Config, text: &str, direction: Direction) -> Result<String, String> {
    let url = completions_url(&config.api_base);
    let body = request_body(&config.model, text, direction);

    let mut request = ureq::post(&url).timeout(Duration::from_secs(60));
    if !config.api_key.is_empty() {
        request = request.set("Authorization", &format!("Bearer {}", config.api_key));
    }
    let response: Value = match request.send_json(body) {
        Ok(r) => r.into_json().map_err(|e| format!("bad JSON from {url}: {e}"))?,
        Err(ureq::Error::Status(code, r)) => {
            let detail = r.into_string().unwrap_or_default();
            return Err(format!("{url} returned {code}: {}", detail.chars().take(200).collect::<String>()));
        }
        Err(e) => return Err(format!("could not reach {url}: {e}")),
    };

    response["choices"][0]["message"]["content"]
        .as_str()
        .map(clean_reply)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            let detail = response.to_string();
            format!("{url} gave no translation: {}", detail.chars().take(200).collect::<String>())
        })
}

/// Models sometimes wrap the answer in quotes or add a thinking block; drop both.
fn clean_reply(raw: &str) -> String {
    let text = match raw.rfind("</think>") {
        Some(end) => &raw[end + "</think>".len()..],
        None => raw,
    };
    let text = text.trim();
    let text = text
        .strip_prefix('"')
        .and_then(|t| t.strip_suffix('"'))
        .unwrap_or(text);
    // WoW chat is single-line.
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;

    #[test]
    fn quest_words_use_wow_meanings() {
        let found = slang_in("我不太熟悉任务路线，谁能带我做任务");
        assert!(found.contains(&("做任务", "questing")));
        assert!(found.contains(&("任务路线", "quest route")));
        assert_eq!(slang_in("还差一个任务"), vec![("任务", "quest")]);
    }

    #[test]
    fn cleans_reply() {
        assert_eq!(clean_reply("\"Need a healer\"\n"), "Need a healer");
        assert_eq!(clean_reply("<think>hmm</think>\nLF tank"), "LF tank");
    }

    #[test]
    fn calls_openai_compatible_endpoint() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut buf = vec![0u8; 8192];
            let n = stream.read(&mut buf).unwrap();
            let request = String::from_utf8_lossy(&buf[..n]).to_string();
            let body = r#"{"choices":[{"message":{"role":"assistant","content":"LF healer for the dungeon"}}]}"#;
            let reply = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
                body.len()
            );
            stream.write_all(reply.as_bytes()).unwrap();
            request
        });
        let config = Config {
            api_base: format!("http://127.0.0.1:{port}/v1/"),
            api_key: "secret".into(),
            ..Config::default()
        };
        let english = translate(&config, "副本缺奶", Direction::ToEnglish).unwrap();
        assert_eq!(english, "LF healer for the dungeon");
        let request = server.join().unwrap();
        assert!(request.starts_with("POST /v1/chat/completions"));
        assert!(request.contains("Bearer secret"));
    }

    #[test]
    fn adds_v1_to_bare_host() {
        assert_eq!(completions_url("http://localhost:1234"), "http://localhost:1234/v1/chat/completions");
        assert_eq!(completions_url("http://localhost:1234/"), "http://localhost:1234/v1/chat/completions");
        assert_eq!(completions_url("http://localhost:1234/v1/"), "http://localhost:1234/v1/chat/completions");
        assert_eq!(completions_url("https://api.openai.com/v1"), "https://api.openai.com/v1/chat/completions");
    }

    #[test]
    fn hy_mt_uses_model_card_prompt_and_glossary() {
        let body = request_body("hf.co/tencent/HY-MT1.5-7B-GGUF:Q4_K_M", "副本来人 缺奶", Direction::ToEnglish);
        let messages = body["messages"].as_array().unwrap();
        assert_eq!(messages.len(), 1, "HY-MT takes no system prompt");
        let prompt = messages[0]["content"].as_str().unwrap();
        assert!(prompt.starts_with("参考下面的翻译：\n"));
        assert!(prompt.contains("副本 翻译成 dungeon"));
        assert!(prompt.contains("缺奶 翻译成 need a healer"));
        assert!(!prompt.contains("坦克"));
        assert!(prompt.ends_with("不要额外解释：\n副本来人 缺奶"));
        assert_eq!(body["top_k"], 20);

        let plain = request_body("HY-MT1.5-7B", "你好", Direction::ToEnglish);
        assert_eq!(
            plain["messages"][0]["content"],
            "将以下文本翻译为英语，注意只需要输出翻译后的结果，不要额外解释：\n\n你好"
        );
        let general = request_body("qwen2.5:7b", "缺奶骑", Direction::ToEnglish);
        assert_eq!(general["messages"][0]["role"], "system");
        let system = general["messages"][0]["content"].as_str().unwrap();
        assert!(system.ends_with("Slang in this line: 缺奶 = need a healer, 奶骑 = holy paladin."), "{system}");
    }

    #[test]
    fn slang_prefers_longest_terms() {
        assert_eq!(slang_in("奶骑来"), vec![("奶骑", "holy paladin")]);
        assert_eq!(slang_in("hello"), vec![]);
        assert!(SLANG.split(';').all(|pair| pair.split_once('=').is_some_and(|(zh, en)| !zh.is_empty() && !en.is_empty())));
    }
}
