use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use walkdir::WalkDir;
use serde::{Deserialize, Serialize};
// ここを修正：Client をしっかりとインポートに含めます
use async_openai::Client; 
use async_openai::types::{
    ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestUserMessageArgs,
    ChatCompletionResponseFormat, ChatCompletionResponseFormatType,
    CreateChatCompletionRequestArgs,
};

// --- 1. LLMから返却されるJSONデータの構造定義 (Serde) ---

#[derive(Serialize, Deserialize, Debug)]
struct TimelineEvent {
    time: String,
    person: String,
    action: String,
    location: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct Relationship {
    from: String,
    to: String,
    label: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct LocationInfo {
    name: String,
    lat_long_sim: String,
    detail: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct Conflict {
    title: String,
    severity: String,
    detail: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct InvestigationReport {
    timeline: Vec<TimelineEvent>,
    relationships: Vec<Relationship>,
    locations: Vec<LocationInfo>,
    conflicts: Vec<Conflict>,
}

// --- 2. 補助関数：フォルダ内の全txtファイルの読み込み ---

fn load_testimonies_from_folder<P: AsRef<Path>>(folder_path: P) -> Result<HashMap<String, String>, std::io::Error> {
    let mut testimonies = HashMap::new();

    for entry in WalkDir::new(folder_path).into_iter().filter_map(|e| e.ok()) {
        if entry.path().extension().and_then(|s| s.to_str()) == Some("txt") {
            let file_name = entry.file_name().to_string_lossy().into_owned();
            let mut file = File::open(entry.path())?;
            let mut content = String::new();
            file.read_to_string(&mut content)?;
            testimonies.insert(file_name, content);
        }
    }
    Ok(testimonies)
}

// --- 3. メイン解析ロジック (非同期) ---
// 一時的なテスト用モック関数（APIを呼ばずにシミュレーション）
async fn analyze_all_testimonies(_files_dict: &HashMap<String, String>, _api_key: &str) -> Result<InvestigationReport, Box<dyn std::error::Error>> {
    println!("（※テストモード：OpenAI APIをバイパスしてモックデータを生成中...）");

    // GPT-4oが本来返してくるはずのJSON文字列を手動で再現
    let mock_json_str = r#"{
      "timeline": [
        {"time": "20:00", "person": "目撃者A", "action": "公園に滞在、黒いジャケットの男を見る", "location": "事件現場近くの公園"},
        {"time": "20:00", "person": "被疑者B", "action": "自宅でテレビ特番を視聴中と主張", "location": "被疑者の自宅"},
        {"time": "20:15", "person": "目撃者A", "action": "男が駅方向へ逃走するのを目撃", "location": "公園前の通り"}
      ],
      "relationships": [
        {"from": "目撃者A", "to": "被疑者B", "label": "面識なし（ただし特徴が一致）"}
      ],
      "locations": [
        {"name": "事件現場近くの公園", "lat_long_sim": "35.6895,139.6917", "detail": "犯行時刻に不審な男の目撃情報あり"}
      ],
      "conflicts": [
        {"title": "アリバイの時間的矛盾", "severity": "高", "detail": "被疑者は自宅にいたと供述しているが、現場から自宅までは車で30分かかるため、20:15の目撃情報が正しければ物理的に移動が不可能。"}
      ]
    }"#;

    // Rustの型安全な構造体にパース（ここが通れば、Serdeの定義が正しい証拠です）
    let report: InvestigationReport = serde_json::from_str(mock_json_str)?;
    Ok(report)
}
/*async fn analyze_all_testimonies(files_dict: &HashMap<String, String>, api_key: &str) -> Result<InvestigationReport, Box<dyn std::error::Error>> {
    // OpenAIクライアントの初期化
    let client = Client::with_config(
        async_openai::config::OpenAIConfig::default().with_api_key(api_key)
    );

    // 全証言のテキストを結合
    let mut combined_content = String::new();
    for (name, content) in files_dict {
        combined_content.push_str(&format!("\n### 証言ファイル: {} ###\n{}\n", name, content));
    }

    let system_prompt = "あなたは高度な犯罪心理捜査官です。提供された全ての証言から以下の4点を抽出し、必ず指定のJSON形式で出力してください。\n\
                         1. timeline: 全証言を統合した時系列\n\
                         2. relationships: 人物間の関係性\n\
                         3. locations: 登場する重要な場所とその意味\n\
                         4. conflicts: 証言間の物理的・論理的な矛盾点";

    let user_prompt = format!(
        "以下の証言データを統合解析してください。\n\n{}\n\n\
        出力形式(JSON):\n\
        {{\n\
          \"timeline\": [{{精度高めの時間表記 \"time\": \"HH:MM\", \"person\": \"名前\", \"action\": \"行動\", \"location\": \"場所\"}}],\n\
          \"relationships\": [{{反転のない構造 \"from\": \"名前\", \"to\": \"名前\", \"label\": \"関係\"}}],\n\
          \"locations\": [{{順不同 \"name\": \"場所名\", \"lat_long_sim\": \"擬似座標\", \"detail\": \"詳細\"}}],\n\
          \"conflicts\": [{{矛盾の検出 \"title\": \"矛盾点\", \"severity\": \"高/中/低\", \"detail\": \"根拠\"}}]\n\
        }}",
        combined_content
    );

    // チャットリクエストの作成 (JSONモードを強制)
    let request = CreateChatCompletionRequestArgs::default()
        .model("gpt-4o")
        .response_format(
            ChatCompletionResponseFormat {
                r#type: ChatCompletionResponseFormatType::JsonObject,
            }
        ) // JSONモードの有効化
        .temperature(0.1)
        .messages([
            ChatCompletionRequestSystemMessageArgs::default().content(system_prompt).build()?.into(),
            ChatCompletionRequestUserMessageArgs::default().content(user_prompt).build()?.into(),
        ])
        .build()?;

    let response = client.chat().create(request).await?;
    let raw_json_str = response.choices[0].message.content.as_ref().ok_or("No content received")?;

    // Rustの型安全な構造体にパース
    let report: InvestigationReport = serde_json::from_str(raw_json_str)?;
    Ok(report)
}*/

// --- 4. エントリーポイント ---

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // APIキーの設定（環境変数から取得するのがRustの推奨スタイルです）
    let api_key = std::env::var("OPENAI_API_KEY")
        .expect("環境変数 'OPENAI_API_KEY' が設定されていません。");

    let folder_path = "./testimonies";
    println!("'{}' フォルダから証言データを読み込んでいます...", folder_path);

    // データの読み込み
    let testimonies = load_testimonies_from_folder(folder_path)?;
    if testimonies.is_empty() {
        println!("テキストファイルが見つかりません。'./testimonies' 内に.txtファイルを配置してください。");
        return Ok(());
    }
    println!("{} 件のファイルを検出しました。解析を開始します...", testimonies.len());

    // 解析実行
    match analyze_all_testimonies(&testimonies, &api_key).await {
        Ok(report) => {
            println!("\n=================================");
            println!("🚨 捜査解析完了結果 (Rust型安全データ)");
            println!("=================================\n");

            println!("--- 📅 統合タイムライン ---");
            for event in report.timeline {
                println!("[{}] {} は {} で [{}] の行動を取った", event.time, event.person, event.location, event.action);
            }

            println!("\n--- 🔍 検出された不適合・矛盾点 ---");
            for conflict in report.conflicts {
                println!("【危険度: {}】 タイトル: {}", conflict.severity, conflict.title);
                println!("詳細の根拠: {}\n", conflict.detail);
            }

            // 必要に応じてここでファイル出力やサーバーへのレスポンス処理を行います
        }
        Err(e) => eprintln!("解析中にエラーが発生しました: {}", e),
    }

    Ok(())
}