//! 卡片語料（spec §2，`FENGSHUI_ROOM_CORPUS_VERSION`）＋所有固定文案
//! （§2.5 開場句與說明卡、§3 翻卡先猜、§4.2 先猜後試）。
//!
//! 狀態：**草稿**——語料定稿前須經 Claude 逐卡兩欄對讀審稿，再給站長（spec §2.6）。
//!
//! 文案紅線（spec §2.6，測試守門）：
//! - 風水欄一律引述句型、只引溫和後果；「煞」只能出現在風水欄。
//! - 研究欄只陳述測了什麼、在什麼結果上、限制；沒有研究就說沒有，不補推想、不用預測處理補故事。
//! - 調整只給不買東西的低成本做法；搬動家具的調整 id 以 `move_` 開頭，且必附「量力而為」。
//! - ok 卡沒有徽章、沒有調整。

use super::observe::Adherence;
use super::{Reach, Status};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Adjustment {
    pub id: &'static str,
    pub text: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Badge {
    pub reach: Reach,
    /// 必須寫出測的是什麼結果（spec §2.2）。
    pub limitation: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CardText {
    pub item_id: &'static str,
    pub status: Status,
    pub variant: Option<&'static str>,
    /// 風水怎麼說（≤100 字）。
    pub fengshui: &'static str,
    /// 研究怎麼說（≤140 字）。
    pub research: &'static str,
    /// 只在 flag／mild 卡。
    pub badge: Option<Badge>,
    /// 「看研究」展開的文獻 key（見 `evidence.rs`）。
    pub refs: &'static [&'static str],
    /// 可以試試看（flag／mild：1–3 個；ok：空）。
    pub adjustments: &'static [Adjustment],
}

impl CardText {
    /// 卡片穩定識別：`itemId:status[:variant]`。
    pub fn key(&self) -> String {
        match self.variant {
            Some(v) => format!("{}:{}:{}", self.item_id, self.status.wire(), v),
            None => format!("{}:{}", self.item_id, self.status.wire()),
        }
    }
}

/// 依 (題目, 狀態, 變體) 查卡。
pub fn card_text(
    item_id: &str,
    status: Status,
    variant: Option<&str>,
) -> Option<&'static CardText> {
    CARDS
        .iter()
        .find(|c| c.item_id == item_id && c.status == status && c.variant == variant)
}

// ── 共用片段 ──

const fn adj(id: &'static str, text: &'static str) -> Adjustment {
    Adjustment { id, text }
}

const fn related(limitation: &'static str) -> Option<Badge> {
    Some(Badge {
        reach: Reach::Related,
        limitation,
    })
}

const fn no_research(limitation: &'static str) -> Option<Badge> {
    Some(Badge {
        reach: Reach::NoResearch,
        limitation,
    })
}

const NO_ADJ: &[Adjustment] = &[];

const ADJ_SWAP_PILLOW: Adjustment = adj("swap_pillow", "試著把枕頭換到床的另一頭，睡幾晚看看。");
const ADJ_CLOSE_DOOR_NIGHT: Adjustment = adj("close_door", "睡覺時把房門關上。");

const R_PROSPECT_REFUGE: &[&str] = &["dosen_ostwald2016"];
const R_CURVE: &[&str] = &["bar_neta2006", "chuquichambi2022", "furniture2025"];
const R_CLUTTER: &[&str] = &["saxbe_repetti2010", "roster2016"];
const R_ENCLOSURE: &[&str] = &["fich2014"];
const R_VIEW: &[&str] = &["stevenson2018", "ulrich1984"];
const R_DAYLIGHT: &[&str] = &["boubekri2014"];
const R_NONE: &[&str] = &[];

const RESEARCH_BED_BACK: &str = "瞭望－庇護理論認為人偏好背後有遮蔽的位置。統合分析整理這類研究後認為結果不一致；這些研究測的是對景觀與座位的偏好評分，目前沒有研究測過床位與睡眠。";
const RESEARCH_BED_CORNER: &str = "曲線偏好的研究顯示，人平均略偏好曲線圖形，但用的多是抽象圖形；2025 年一篇用真實桌椅的研究沒有重現這個偏好。目前沒有研究測過尖角與睡眠。";
const RESEARCH_SEAT_CORNER: &str = "曲線偏好的研究顯示，人平均略偏好曲線圖形，但用的多是抽象圖形；2025 年一篇用真實桌椅的研究沒有重現這個偏好。目前沒有研究測過尖角與專注。";
const RESEARCH_BED_CLUTTER: &str = "有研究分析人導覽自家時的描述用語，把家說得較雜亂的人，一天中的壓力荷爾蒙型態與情緒較差；另有問卷研究發現雜物程度與幸福感有關。兩者都只有相關、不能推論因果，也沒有測過床底。";
const RESEARCH_SEAT_CLUTTER: &str = "有研究分析人導覽自家時的描述用語，把家說得較雜亂的人，一天中的壓力荷爾蒙型態與情緒較差；另有問卷研究發現雜物程度與幸福感有關。兩者都只有相關、不能推論因果，也沒有測過桌面。";
const RESEARCH_BEAM: &str = "目前沒有研究直接測過橫樑。有一篇虛擬實境研究比較開放與封閉的房間對壓力反應的影響，操弄的是房間開口，不是樑，這裡只列為延伸閱讀。";
const RESEARCH_BED_LIGHT: &str = "有研究比較有窗與無窗辦公室的員工，有窗的人睡眠自評較好，測的是白天光照。夜間光線與睡眠的具名文獻還在整理；也沒有研究測過明廳暗房這個說法。";
const RESEARCH_SEAT_LIGHT: &str = "有研究比較有窗與無窗辦公室的員工，有窗的人睡眠自評較好，但樣本小、不是隨機分組。目前沒有研究直接測過座位自然光與專心。";
const RESEARCH_SEAT_BACK: &str = "瞭望－庇護理論認為人偏好背後有遮蔽的座位。統合分析整理這類研究後認為結果不一致；這些研究測的是座位偏好評分，不是壓力或專注。";
const RESEARCH_SEAT_FRONT: &str = "注意力恢復研究的統合分析顯示，接觸自然環境對注意力測驗有小幅效果，研究之間差異大；另有醫院研究比較窗外看得到樹的病人。這些研究都沒有測過座位面牆。";

const LIM_BED_BACK: &str = "測的是景觀與座位偏好，結果不一致；沒有床位研究";
const LIM_CORNER: &str = "測的是圖形偏好評分，真實家具研究沒有重現";
const LIM_CLUTTER: &str = "測的是對整個家的描述與壓力荷爾蒙、情緒，只有相關";
const LIM_SEAT_BACK: &str = "測的是座位偏好評分，不是壓力或專注";

pub static CARDS: &[CardText] = &[
  // ═══ 臥室 ═══

  // ── bed_back ──
  CardText {
    item_id: "bed_back",
    status: Status::Ok,
    variant: None,
    fengshui: "風水的說法是床頭要靠實牆，稱為「有靠」，睡起來比較安穩。你的床頭靠著實牆，符合這個常見建議。",
    research: RESEARCH_BED_BACK,
    badge: None,
    refs: R_PROSPECT_REFUGE,
    adjustments: NO_ADJ,
  },
  CardText {
    item_id: "bed_back",
    status: Status::Flag,
    variant: Some("window"),
    fengshui: "風水的說法是床頭靠窗等於「沒有靠」，風水師常認為容易睡不安穩、心神不寧；也有人只建議晚上把窗簾拉上。各家說法不一。",
    research: RESEARCH_BED_BACK,
    badge: related(LIM_BED_BACK),
    refs: R_PROSPECT_REFUGE,
    adjustments: &[
      adj("draw_curtain", "睡前把現有的窗簾拉好，讓床頭後面多一層遮擋。"),
      ADJ_SWAP_PILLOW,
      adj("move_bed_wall", "如果其他牆面放得下，可以把床頭移到實牆邊；量力而為，不要獨自搬重物。"),
    ],
  },
  CardText {
    item_id: "bed_back",
    status: Status::Flag,
    variant: Some("passage"),
    fengshui: "風水的說法是床頭後面是門或走道，背後有人走動、沒有依靠，睡覺時不易放鬆、睡不安穩。",
    research: RESEARCH_BED_BACK,
    badge: related(LIM_BED_BACK),
    refs: R_PROSPECT_REFUGE,
    adjustments: &[
      adj("close_back_door", "睡覺時把床頭後面的門關上。"),
      ADJ_SWAP_PILLOW,
      adj("move_bed_wall", "如果其他牆面放得下，可以把床頭移到實牆邊；量力而為，不要獨自搬重物。"),
    ],
  },
  CardText {
    item_id: "bed_back",
    status: Status::Flag,
    variant: Some("floating"),
    fengshui: "風水的說法是床頭沒靠牆、四面懸空，等於「沒有靠」，容易睡不安穩、心神不寧。",
    research: RESEARCH_BED_BACK,
    badge: related(LIM_BED_BACK),
    refs: R_PROSPECT_REFUGE,
    adjustments: &[
      adj("move_bed_push", "把床頭推近最近的實牆；量力而為，不要獨自搬重物。"),
      adj("move_cabinet_back", "用現有的矮櫃或書櫃擋在床頭後方；量力而為，不要獨自搬重物。"),
    ],
  },
  // ── bed_door_view ──
  CardText {
    item_id: "bed_door_view",
    status: Status::Ok,
    variant: None,
    fengshui: "風水師常建議躺在床上看得到房門、但床不要正對門，門在斜前方是常見的理想位置。你的配置符合這個常見建議。",
    research: "有些研究顯示人偏好看得到入口的位置，但結果不一致；這些研究測的是座位與景觀偏好，不是睡眠。",
    badge: None,
    refs: R_PROSPECT_REFUGE,
    adjustments: NO_ADJ,
  },
  CardText {
    item_id: "bed_door_view",
    status: Status::Flag,
    variant: Some("facing"),
    fengshui: "風水的說法是床尾不要正對房門，傳統上帶有不好的聯想；風水師常建議避開，認為容易睡不安穩。各家理由不一。",
    research: "目前沒有研究直接測過床尾對門。",
    badge: no_research("沒有研究測過床尾與房門的相對位置，也沒有睡眠資料"),
    refs: R_NONE,
    adjustments: &[
      ADJ_CLOSE_DOOR_NIGHT,
      adj("move_bed_shift", "把床稍微挪開，讓床尾不正對門口；量力而為，不要獨自搬重物。"),
    ],
  },
  CardText {
    item_id: "bed_door_view",
    status: Status::Flag,
    variant: Some("hidden"),
    fengshui: "風水的說法是躺著看不到房門，對進出的動靜沒有掌握，容易心神不寧、不易放鬆。",
    research: "瞭望－庇護研究裡「看得到入口」這一半，支持的結果比「背後有遮蔽」稍多，但整體仍不一致；測的是座位與景觀偏好評分，目前沒有研究測過床位與睡眠。",
    badge: related("測的是座位與景觀偏好，不是睡眠；結果不一致"),
    refs: R_PROSPECT_REFUGE,
    adjustments: &[
      ADJ_CLOSE_DOOR_NIGHT,
      adj("turn_head", "換個方向睡幾晚，讓躺著時門在視線內。"),
    ],
  },
  // ── bed_overhead ──
  CardText {
    item_id: "bed_overhead",
    status: Status::Ok,
    variant: None,
    fengshui: "風水的說法是床的正上方不要有橫樑或重物。你的床上方淨空，符合這個常見建議。",
    research: RESEARCH_BEAM,
    badge: None,
    refs: R_ENCLOSURE,
    adjustments: NO_ADJ,
  },
  CardText {
    item_id: "bed_overhead",
    status: Status::Flag,
    variant: Some("beam"),
    fengshui: "風水稱為「樑壓床」，也有人叫「壓樑煞」，說法是樑橫在床上方讓人有壓迫感、睡不安穩。樑的位置要多正才算，各家說法不一。",
    research: RESEARCH_BEAM,
    badge: no_research("沒有研究測過橫樑；封閉空間研究操弄的是房間開口"),
    refs: R_ENCLOSURE,
    adjustments: &[
      ADJ_SWAP_PILLOW,
      adj("move_bed_beam", "把床挪到樑不在正上方的位置；量力而為，不要獨自搬重物。"),
    ],
  },
  CardText {
    item_id: "bed_overhead",
    status: Status::Flag,
    variant: Some("heavy"),
    fengshui: "風水的說法是頭頂上方有吊櫃、層板或大型吊燈，像有東西懸在上方，容易心神不寧、不易放鬆。",
    research: "目前沒有研究直接測過床上方的吊櫃、層板或吊燈。有一篇虛擬實境研究比較開放與封閉的房間，操弄的是房間開口，這裡只列為延伸閱讀。",
    badge: no_research("沒有研究測過頭頂重物；封閉空間研究操弄的是房間開口"),
    refs: R_ENCLOSURE,
    adjustments: &[
      adj("lower_items", "把層板或吊櫃上較重的東西移到低處。"),
      ADJ_SWAP_PILLOW,
    ],
  },
  // ── bed_mirror ──
  CardText {
    item_id: "bed_mirror",
    status: Status::Ok,
    variant: None,
    fengshui: "風水的說法是鏡子不要照到床上的人，理由各家說法不一。你的鏡子照不到床，符合這個常見建議。",
    research: "目前沒有研究直接測過鏡子和睡眠。",
    badge: None,
    refs: R_NONE,
    adjustments: NO_ADJ,
  },
  CardText {
    item_id: "bed_mirror",
    status: Status::Flag,
    variant: None,
    fengshui: "風水的說法是躺著時鏡子照得到自己，半夜醒來容易被鏡中的影像嚇到、睡不安穩。各家說法不一。",
    research: "目前沒有研究直接測過臥室鏡子和睡眠。",
    badge: no_research("沒有研究測過臥室鏡子與睡眠"),
    refs: R_NONE,
    adjustments: &[
      adj("cover_mirror", "睡前用現有的布或毛巾把鏡子蓋起來。"),
      adj("turn_mirror", "如果是可移動的立鏡，轉個方向讓它照不到床。"),
    ],
  },
  // ── bed_corner ──
  CardText {
    item_id: "bed_corner",
    status: Status::Ok,
    variant: None,
    fengshui: "風水的說法是柱角、牆角或櫃角的尖端不要朝著床。你的床邊沒有這樣的尖角，符合這個常見建議。",
    research: RESEARCH_BED_CORNER,
    badge: None,
    refs: R_CURVE,
    adjustments: NO_ADJ,
  },
  CardText {
    item_id: "bed_corner",
    status: Status::Flag,
    variant: None,
    fengshui: "風水稱尖角朝人為「壁刀」或「角煞」，說法是尖角對著床，容易心神不寧、不易放鬆。",
    research: RESEARCH_BED_CORNER,
    badge: related(LIM_CORNER),
    refs: R_CURVE,
    adjustments: &[
      adj("move_cabinet_turn", "把櫃子稍微轉向或挪開，讓角不朝著床；量力而為，不要獨自搬重物。"),
      adj("block_corner", "用現有的盆栽或小家具擋在尖角前面。"),
    ],
  },
  // ── bed_door_door ──
  CardText {
    item_id: "bed_door_door",
    status: Status::Ok,
    variant: None,
    fengshui: "風水的說法是房門不要和其他房門正對。你的房門沒有正對其他門，符合這個常見建議。",
    research: "目前沒有研究直接測過門對門。",
    badge: None,
    refs: R_NONE,
    adjustments: NO_ADJ,
  },
  CardText {
    item_id: "bed_door_door",
    status: Status::Flag,
    variant: None,
    fengshui: "風水的說法是門對門彼此直通，進出的動靜互相干擾，讓人不易放鬆。嚴重程度各家說法不一。",
    research: "目前沒有研究直接測過門對門。",
    badge: no_research("沒有研究測過門對門的配置"),
    refs: R_NONE,
    adjustments: &[
      adj("close_door_daily", "平常把房門關上。"),
      adj("hang_cloth", "如果家裡有現成的布簾，可以掛在房門內側。"),
    ],
  },
  // ── bed_light ──
  CardText {
    item_id: "bed_light",
    status: Status::Ok,
    variant: None,
    fengshui: "風水常說「明廳暗房」，臥室宜暗。風水講的明廳暗房不是指夜間遮光，這裡問的是相近但不同的事。你的房間晚上能暗下來。",
    research: RESEARCH_BED_LIGHT,
    badge: None,
    refs: R_DAYLIGHT,
    adjustments: NO_ADJ,
  },
  CardText {
    item_id: "bed_light",
    status: Status::Flag,
    variant: None,
    fengshui: "風水常說「明廳暗房」，臥室宜暗、利於休息。風水講的明廳暗房不是指夜間遮光，這裡問的是相近但不同的事。",
    research: RESEARCH_BED_LIGHT,
    badge: related("相關研究測的是白天光照與睡眠；題目與風水原意不同"),
    refs: R_DAYLIGHT,
    adjustments: &[
      adj("draw_curtain_tight", "睡前把現有的窗簾拉緊，縫隙用夾子夾住。"),
      adj("cover_leds", "把會發光的充電器、指示燈轉向或用東西蓋住。"),
    ],
  },
  // ── bed_clutter ──
  CardText {
    item_id: "bed_clutter",
    status: Status::Ok,
    variant: None,
    fengshui: "風水師常建議地面和床底保持清爽，讓空間流通。你的房間雜物很少，符合這個常見建議。",
    research: RESEARCH_BED_CLUTTER,
    badge: None,
    refs: R_CLUTTER,
    adjustments: NO_ADJ,
  },
  CardText {
    item_id: "bed_clutter",
    status: Status::Mild,
    variant: None,
    fengshui: "風水師常建議地面和床底保持清爽，認為雜物堆積讓空間不流通、不易放鬆。你的房間有一些雜物。",
    research: RESEARCH_BED_CLUTTER,
    badge: related(LIM_CLUTTER),
    refs: R_CLUTTER,
    adjustments: &[
      adj("tidy_corner", "挑床邊一小區，先整理十分鐘就好。"),
      adj("clear_under_bed", "把床底的東西清出一格空位。"),
    ],
  },
  CardText {
    item_id: "bed_clutter",
    status: Status::Flag,
    variant: None,
    fengshui: "風水師常建議地面和床底保持清爽，認為雜物堆積讓空間不流通、不易放鬆、睡不安穩。你的房間雜物很多。",
    research: RESEARCH_BED_CLUTTER,
    badge: related(LIM_CLUTTER),
    refs: R_CLUTTER,
    adjustments: &[
      adj("tidy_corner", "先整理床邊一小區，不用一次做完。"),
      adj("group_under_bed", "把床底不常用的東西集中到同一個現有的箱子裡。"),
      adj("clear_path", "先清出從房門走到床邊的一條路。"),
    ],
  },
  // ═══ 書房／工作位 ═══

  // ── seat_back ──
  CardText {
    item_id: "seat_back",
    status: Status::Ok,
    variant: None,
    fengshui: "風水的說法是座位背後要有實牆，稱為「有靠」，坐得比較安心。你的座位背後是實牆，符合這個常見建議。",
    research: RESEARCH_SEAT_BACK,
    badge: None,
    refs: R_PROSPECT_REFUGE,
    adjustments: NO_ADJ,
  },
  CardText {
    item_id: "seat_back",
    status: Status::Flag,
    variant: Some("window"),
    fengshui: "風水的說法是座位背後是窗等於「沒有靠」，容易心神不寧、不易專心。也有人認為只要窗簾拉上就好，各家說法不一。",
    research: RESEARCH_SEAT_BACK,
    badge: related(LIM_SEAT_BACK),
    refs: R_PROSPECT_REFUGE,
    adjustments: &[
      adj("half_curtain", "工作時把背後現有的窗簾拉上一半。"),
      adj("move_desk_wall", "試著把桌椅轉個方向，讓背後靠牆；量力而為，不要獨自搬重物。"),
    ],
  },
  CardText {
    item_id: "seat_back",
    status: Status::Flag,
    variant: Some("passage"),
    fengshui: "風水的說法是座位背後是門或走道，身後有人走動，容易分心、不易放鬆。",
    research: RESEARCH_SEAT_BACK,
    badge: related(LIM_SEAT_BACK),
    refs: R_PROSPECT_REFUGE,
    adjustments: &[
      adj("close_back_door", "工作時把背後的門關上。"),
      adj("move_shelf_back", "把現有的書櫃或櫃子移到座位後方；量力而為，不要獨自搬重物。"),
    ],
  },
  // ── seat_door_view ──
  CardText {
    item_id: "seat_door_view",
    status: Status::Ok,
    variant: None,
    fengshui: "風水師常建議坐著時房門在視線內、但不要正對門。你的座位符合這個常見建議。",
    research: "有些研究顯示人偏好看得到入口的座位，但結果不一致；這些研究測的是座位偏好評分，不是專注。",
    badge: None,
    refs: R_PROSPECT_REFUGE,
    adjustments: NO_ADJ,
  },
  CardText {
    item_id: "seat_door_view",
    status: Status::Flag,
    variant: Some("facing"),
    fengshui: "風水的說法是座位正對房門，進出的人直衝而來，容易分心、心神不寧。",
    research: "目前沒有研究直接測過座位正對門。",
    badge: no_research("沒有研究測過座位正對門，也沒有專注方面的資料"),
    refs: R_NONE,
    adjustments: &[
      adj("close_door_work", "工作時把房門關上。"),
      adj("move_desk_angle", "把桌子稍微斜放，讓門在斜前方；量力而為，不要獨自搬重物。"),
    ],
  },
  CardText {
    item_id: "seat_door_view",
    status: Status::Flag,
    variant: Some("hidden"),
    fengshui: "風水的說法是背對門坐，看不到身後的進出，容易心神不寧、不易專心。",
    research: "瞭望－庇護研究裡「看得到入口」這一半，支持的結果比「背後有遮蔽」稍多，但整體仍不一致；測的是座位偏好評分，不是壓力或專注。",
    badge: related(LIM_SEAT_BACK),
    refs: R_PROSPECT_REFUGE,
    adjustments: &[
      adj("close_door_work", "工作時把房門關上。"),
      adj("move_desk_turn", "把桌子轉個方向，讓門進入視線；量力而為，不要獨自搬重物。"),
    ],
  },
  // ── seat_overhead ──
  CardText {
    item_id: "seat_overhead",
    status: Status::Ok,
    variant: None,
    fengshui: "風水的說法是座位上方不要有橫樑。你的座位上方沒有外露的樑，符合這個常見建議。",
    research: RESEARCH_BEAM,
    badge: None,
    refs: R_ENCLOSURE,
    adjustments: NO_ADJ,
  },
  CardText {
    item_id: "seat_overhead",
    status: Status::Flag,
    variant: None,
    fengshui: "風水稱為「樑壓」，也有人叫「壓樑煞」，說法是坐在樑下讓人有壓迫感、容易分心。各家說法不一。",
    research: RESEARCH_BEAM,
    badge: no_research("沒有研究測過橫樑；封閉空間研究操弄的是房間開口"),
    refs: R_ENCLOSURE,
    adjustments: &[
      adj("shift_chair", "把椅子往旁邊挪一點，坐的位置避開樑的正下方。"),
      adj("move_desk_beam", "把桌子移到樑不在正上方的位置；量力而為，不要獨自搬重物。"),
    ],
  },
  // ── seat_front ──
  CardText {
    item_id: "seat_front",
    status: Status::Ok,
    variant: None,
    fengshui: "風水稱座位前方為「明堂」，說法是前方開闊、視野寬，心情比較開朗。你的座位前方開闊，符合這個常見建議。",
    research: RESEARCH_SEAT_FRONT,
    badge: None,
    refs: R_VIEW,
    adjustments: NO_ADJ,
  },
  CardText {
    item_id: "seat_front",
    status: Status::Flag,
    variant: None,
    fengshui: "風水稱座位前方為「明堂」，說法是前方直接面對很近的牆，明堂太窄，容易覺得侷促、不易專心。",
    research: RESEARCH_SEAT_FRONT,
    badge: related("測的是自然景觀與注意力，效果小、差異大；沒有測面牆"),
    refs: R_VIEW,
    adjustments: &[
      adj("clear_wall", "把桌前牆面上的東西清掉一些，留一點空白。"),
      adj("move_desk_open", "把桌子轉向窗或房間較開闊的一側；量力而為，不要獨自搬重物。"),
    ],
  },
  // ── seat_corner ──
  CardText {
    item_id: "seat_corner",
    status: Status::Ok,
    variant: None,
    fengshui: "風水的說法是尖角不要朝著座位。你的座位旁邊沒有尖角朝著你，符合這個常見建議。",
    research: RESEARCH_SEAT_CORNER,
    badge: None,
    refs: R_CURVE,
    adjustments: NO_ADJ,
  },
  CardText {
    item_id: "seat_corner",
    status: Status::Flag,
    variant: None,
    fengshui: "風水稱尖角朝人為「壁刀」或「角煞」，說法是尖角對著座位，容易心神不寧、不易專心。",
    research: RESEARCH_SEAT_CORNER,
    badge: related(LIM_CORNER),
    refs: R_CURVE,
    adjustments: &[
      adj("shift_chair_corner", "把椅子挪一點，讓尖角不正對著你。"),
      adj("block_corner", "用現有的盆栽或小家具擋在尖角前面。"),
    ],
  },
  // ── seat_light ──
  CardText {
    item_id: "seat_light",
    status: Status::Ok,
    variant: None,
    fengshui: "風水師常建議書桌要採光充足、明亮。你的座位白天自然光夠，符合這個常見建議。",
    research: RESEARCH_SEAT_LIGHT,
    badge: None,
    refs: R_DAYLIGHT,
    adjustments: NO_ADJ,
  },
  CardText {
    item_id: "seat_light",
    status: Status::Flag,
    variant: None,
    fengshui: "風水師常建議書桌要採光充足、明亮，認為光線昏暗讓人精神不振、不易專心。",
    research: RESEARCH_SEAT_LIGHT,
    badge: related("測的是有窗與睡眠，樣本小；沒有測專心"),
    refs: R_DAYLIGHT,
    adjustments: &[
      adj("open_curtain", "白天工作時把窗簾拉開。"),
      adj("move_desk_window", "把座位往窗邊挪一點；量力而為，不要獨自搬重物。"),
    ],
  },
  // ── seat_clutter ──
  CardText {
    item_id: "seat_clutter",
    status: Status::Ok,
    variant: None,
    fengshui: "風水師常建議桌面和周圍保持清爽。你的座位雜物很少，符合這個常見建議。",
    research: RESEARCH_SEAT_CLUTTER,
    badge: None,
    refs: R_CLUTTER,
    adjustments: NO_ADJ,
  },
  CardText {
    item_id: "seat_clutter",
    status: Status::Mild,
    variant: None,
    fengshui: "風水師常建議桌面和周圍保持清爽，認為雜物堆積讓人容易分心、不易專心。你的座位有一些雜物。",
    research: RESEARCH_SEAT_CLUTTER,
    badge: related(LIM_CLUTTER),
    refs: R_CLUTTER,
    adjustments: &[
      adj("clear_desk_zone", "先清出桌面正前方一個手臂寬的空間。"),
      adj("tidy_ten_min", "收工前花十分鐘，把桌面的東西歸位。"),
    ],
  },
  CardText {
    item_id: "seat_clutter",
    status: Status::Flag,
    variant: None,
    fengshui: "風水師常建議桌面和周圍保持清爽，認為雜物堆積讓人容易分心、心神不寧。你的座位雜物很多。",
    research: RESEARCH_SEAT_CLUTTER,
    badge: related(LIM_CLUTTER),
    refs: R_CLUTTER,
    adjustments: &[
      adj("clear_desk_zone", "先清出桌面正前方一個手臂寬的空間。"),
      adj("group_items", "把散在桌上的東西先集中到一個現有的盒子裡。"),
      adj("tidy_floor", "座位腳邊的東西先移開，不用一次整理完。"),
    ],
  },
];

// ═══ 固定文案 ═══

// ── 介面標籤 ──
pub const LABEL_FENGSHUI: &str = "風水怎麼說";
pub const LABEL_RESEARCH: &str = "研究怎麼說";
pub const LABEL_ADJUSTMENTS: &str = "可以試試看";
pub const LABEL_SEE_RESEARCH: &str = "看研究";
pub const LABEL_OK_GROUP: &str = "已符合常見建議";
pub const LABEL_UNKNOWN_GROUP: &str = "還不確定的項目";
pub const LABEL_LOCAL_ONLY: &str = "答案只存在這個瀏覽器，不會上傳";
pub const LABEL_CLEAR_ANSWERS: &str = "清除我的房間答案";

// ── §2.5 開場一句 ──
pub const OPENING: &str = "每一項會並列風水的說法和相關研究。下面這些研究多數不是為了檢驗風水而做，有些項目根本沒有人研究過。";

// ── §2.5 結果最後的說明卡 ──
pub const EXPLAINER_TITLE: &str = "關於這些說法和研究";
pub const EXPLAINER: [&str; 4] = [
  "目前唯一直接比較過風水佈置的實驗，是 Zijlstra 等人 2024 年發表在 PLOS ONE 的研究：558 人在線上看虛擬病房，風水佈置讓房間顯得較愉悅，對焦慮沒有直接效果。限制是線上進行、看的是虛擬房間。",
  "「預測大腦」是一種看待心智的理論：大腦不斷預測接下來會發生什麼，再拿實際情況來對答案。這裡只把它當成解讀框架，它不是證據，也沒有研究用它檢驗過任何一條風水說法。",
  "你剛剛翻卡前的猜測和答案的落差，就是一次預測誤差。",
  "你的預期本身也會影響你的感受。想自己試一個調整時，可以先把預期寫下來，再回頭對照。",
];

// ── §3 翻卡先猜 ──
pub const REVEAL_PROMPT: &str = "這條說法，研究做到哪了？";
pub const REVEAL_FLIP: &str = "翻開";
pub const REVEAL_HIT: &str = "猜對了";
pub const REVEAL_MISS: &str = "猜錯了";
pub const REVEAL_SKIP_ALL: &str = "全部直接翻開";

/// 猜錯時的一句（spec §3-2）。
pub fn reveal_miss_line(guess: Reach, actual: Reach) -> String {
    format!(
        "你原本以為「{}」，實際是「{}」。",
        guess.guess_label(),
        actual.guess_label()
    )
}

/// 全部翻完的總結（spec §3-3）。
pub fn reveal_score_line(correct: usize, total: usize) -> String {
    format!("你猜對 {correct} / {total} 張")
}

// ── §4 先猜後試 ──
pub const OBS_SECTION: &str = "我的觀察";
pub const OBS_KIND: &str = "自我觀察";
pub const OBS_TRY: &str = "試試看";
pub const OBS_DO_FIRST: &str = "先做完這個調整，再按開始";
pub const OBS_START: &str = "開始";
pub const OBS_LOCAL_ONLY: &str = "觀察紀錄只存在這個瀏覽器，不會上傳，也不會跨裝置同步。";
pub const OBS_MASK_NOTE: &str =
    "回報時先不顯示你的起點和預測，是為了減少被先前的數字牽著走，不是保密。";
pub const OBS_ONE_PER_ROOM: &str = "每個房型同時只能有一個進行中的觀察。";
pub const OBS_BASELINE_INTRO: &str = "先回想調整前的過去 7 天。";
pub const OBS_Q_SAFETY: &str = "待在這個位置你覺得多安心？";
pub const OBS_Q_SAFETY_ANCHORS: (&str, &str) = ("0 很不安心", "10 很安心");
pub const OBS_Q_SLEEP: &str = "睡得好嗎？";
pub const OBS_Q_SLEEP_ANCHORS: (&str, &str) = ("0 很差", "10 很好");
pub const OBS_Q_FOCUS: &str = "能專心嗎？";
pub const OBS_Q_FOCUS_ANCHORS: (&str, &str) = ("0 很難", "10 很容易");
pub const OBS_PREDICT_PROMPT: &str = "調整後這段時間，你猜會落在幾分到幾分？";
pub const OBS_PREDICT_HINT: &str = "可以猜維持不變，也可以猜變差。";
pub const OBS_EXPIRED: &str = "已過期：超過 30 天沒有回報。";
pub const OBS_REPORT_INTRO: &str = "回想調整後的這段時間。";
pub const OBS_ADHERENCE_PROMPT: &str = "這段時間你有維持這個調整嗎？";
pub const OBS_RESULT_IN: &str = "落在你猜的區間內";
pub const OBS_RESULT_OUT: &str = "落在你猜的區間外";
pub const OBS_RESULT_NOTE: &str = "預測和實際的落差，可以當作一個觀察線索。一次的結果分不出是調整、你的預期、生活上的其他變化，還是評分本身的起伏造成的，所以它不能說明這個調整有沒有用，也不能說明風水對不對。";
pub const OBS_WINDOW_NOTE: &str =
    "滿 7 天開放回報、30 天內都可以回報；這只是回訪時間，不代表 7 天足以判斷效果。";
pub const OBS_CLEAR: &str = "清除我的觀察";

/// 回報的維持選項文字。
pub fn adherence_label(a: Adherence) -> &'static str {
    match a {
        Adherence::Yes => "有",
        Adherence::Partial => "部分",
        Adherence::No => "沒有",
    }
}

/// 開放倒數（`days` 由 `observe::days_until_open` 算出）。
pub fn open_countdown_line(days: i64, open_date: &str) -> String {
    if days <= 0 {
        "現在可以回報了。".to_string()
    } else {
        format!("還有 {days} 天，{open_date} 起可以回報。")
    }
}

/// 對答案後的固定說明（spec §4.2-7）。沒有維持調整時改用平常起伏的文案；
/// `delta` = 實際 − 起點。
pub fn observation_note(adherence: Adherence, delta: i32) -> String {
    match adherence {
        Adherence::No if delta == 0 => {
            "這段時間你沒有調整，分數也沒有變。平常的起伏，要多看幾次才知道有多大。".to_string()
        }
        Adherence::No => {
            format!("這段時間你沒有調整，分數還是變了 {delta:+}。這就是平常的起伏有多大。")
        }
        Adherence::Yes | Adherence::Partial => OBS_RESULT_NOTE.to_string(),
    }
}

/// 校準紀錄（spec §4.2-8）；沒有任何紀錄時 `None`。
pub fn calibration_line(hits: u32, total: u32) -> Option<String> {
    (total > 0).then(|| format!("你的預測 {total} 次中有 {hits} 次落在區間內"))
}

/// 所有不在卡片裡的固定文案（T3 掃描用；新增固定文案必須加進來）。
pub fn fixed_texts() -> Vec<String> {
    let mut v: Vec<String> = [
        LABEL_FENGSHUI,
        LABEL_RESEARCH,
        LABEL_ADJUSTMENTS,
        LABEL_SEE_RESEARCH,
        LABEL_OK_GROUP,
        LABEL_UNKNOWN_GROUP,
        LABEL_LOCAL_ONLY,
        LABEL_CLEAR_ANSWERS,
        OPENING,
        EXPLAINER_TITLE,
        REVEAL_PROMPT,
        REVEAL_FLIP,
        REVEAL_HIT,
        REVEAL_MISS,
        REVEAL_SKIP_ALL,
        OBS_SECTION,
        OBS_KIND,
        OBS_TRY,
        OBS_DO_FIRST,
        OBS_START,
        OBS_LOCAL_ONLY,
        OBS_MASK_NOTE,
        OBS_ONE_PER_ROOM,
        OBS_BASELINE_INTRO,
        OBS_Q_SAFETY,
        OBS_Q_SAFETY_ANCHORS.0,
        OBS_Q_SAFETY_ANCHORS.1,
        OBS_Q_SLEEP,
        OBS_Q_SLEEP_ANCHORS.0,
        OBS_Q_SLEEP_ANCHORS.1,
        OBS_Q_FOCUS,
        OBS_Q_FOCUS_ANCHORS.0,
        OBS_Q_FOCUS_ANCHORS.1,
        OBS_PREDICT_PROMPT,
        OBS_PREDICT_HINT,
        OBS_EXPIRED,
        OBS_REPORT_INTRO,
        OBS_ADHERENCE_PROMPT,
        OBS_RESULT_IN,
        OBS_RESULT_OUT,
        OBS_RESULT_NOTE,
        OBS_WINDOW_NOTE,
        OBS_CLEAR,
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    v.extend(EXPLAINER.iter().map(|s| s.to_string()));
    for r in Reach::ALL {
        v.push(r.badge_label().to_string());
        v.push(r.guess_label().to_string());
        for g in Reach::ALL {
            v.push(reveal_miss_line(g, r));
        }
    }
    v.push(reveal_score_line(3, 5));
    for a in [Adherence::Yes, Adherence::Partial, Adherence::No] {
        v.push(adherence_label(a).to_string());
        for d in [-3, 0, 2] {
            v.push(observation_note(a, d));
        }
    }
    v.push(open_countdown_line(0, "2026-11-24"));
    v.push(open_countdown_line(3, "2026-11-24"));
    v.extend(calibration_line(3, 5));
    for room in super::Room::ALL {
        v.push(room.label().to_string());
    }
    for q in super::QUESTIONS {
        v.push(q.prompt.to_string());
        v.extend(q.options.iter().map(|o| o.label.to_string()));
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fengshui::evidence::{evidence_by_key, EVIDENCE};
    use crate::fengshui::QUESTIONS;
    use std::collections::HashSet;

    // ── T2 語料完整 ──

    #[test]
    fn t2_every_non_unknown_option_has_exactly_one_card() {
        for q in QUESTIONS {
            for o in q.options.iter().filter(|o| o.status != Status::Unknown) {
                let n = CARDS
                    .iter()
                    .filter(|c| c.item_id == q.id && c.status == o.status && c.variant == o.variant)
                    .count();
                assert_eq!(n, 1, "{}:{} needs exactly one card", q.id, o.id);
            }
        }
    }

    #[test]
    fn t2_no_orphan_cards() {
        let mut keys = HashSet::new();
        for c in CARDS {
            assert!(keys.insert(c.key()), "duplicate card {}", c.key());
            assert_ne!(c.status, Status::Unknown, "{}", c.key());
            let q = crate::fengshui::question(c.item_id).expect("card for unknown question");
            assert!(
                q.options
                    .iter()
                    .any(|o| o.status == c.status && o.variant == c.variant),
                "orphan card {}",
                c.key()
            );
        }
    }

    #[test]
    fn t2_adjustments_count_and_unique_ids() {
        for c in CARDS {
            if c.status.actionable() {
                assert!(
                    (1..=3).contains(&c.adjustments.len()),
                    "{} needs 1–3 adjustments",
                    c.key()
                );
            } else {
                assert!(
                    c.adjustments.is_empty(),
                    "{} (ok) must not have adjustments",
                    c.key()
                );
            }
            let mut ids = HashSet::new();
            for a in c.adjustments {
                assert!(
                    ids.insert(a.id),
                    "{}: duplicate adjustment {}",
                    c.key(),
                    a.id
                );
                assert!(!a.text.is_empty());
            }
        }
    }

    #[test]
    fn t2_furniture_moves_carry_safety_note() {
        for c in CARDS {
            for a in c.adjustments {
                if a.id.starts_with("move_") {
                    assert!(
                        a.text.contains("量力而為，不要獨自搬重物"),
                        "{}:{} moves furniture without safety note",
                        c.key(),
                        a.id
                    );
                }
            }
        }
    }

    #[test]
    fn t2_card_count() {
        // 臥室 21 張 + 書房 17 張
        assert_eq!(CARDS.len(), 38);
    }

    // ── T3 禁用詞、白名單、「煞」位置 ──

    const BANNED: &[&str] = &[
        // 背書
        "證實",
        "證明",
        "科學解釋",
        "科學根據",
        "研究印證",
        "腦科學支持",
        "所以風水",
        "磁場",
        "能量",
        // 恐嚇
        "凶",
        "不吉",
        "不祥",
        "血光",
        "破財",
        "漏財",
        "財運",
        "離婚",
        "影響婚姻",
        "生病",
        "有損健康",
        "災厄",
        "意外",
        "運勢",
        "出殯",
        "相罵",
        "小人",
        "口舌",
        "陰氣",
        "衰",
        "犯小人",
        "招",
        // 化煞商品
        "八卦鏡",
        "山海鎮",
        "五帝錢",
        "葫蘆",
        "水晶",
        "貔貅",
        "風鈴",
        "化解",
        "開運",
        "旺",
        // 理氣
        "命卦",
        "吉方",
        "財位",
        "文昌",
        "桃花",
        "流年",
        "飛星",
        "羅盤",
        "五行",
        "缺角",
    ];

    const ALLOWED_PHRASES: &[&str] = &["不能證明", "不是在證明", "沒有證實", "不意外"];

    fn banned_hits(text: &str) -> Vec<&'static str> {
        let mut masked = text.to_string();
        for p in ALLOWED_PHRASES {
            masked = masked.replace(p, "□");
        }
        BANNED
            .iter()
            .copied()
            .filter(|w| masked.contains(w))
            .collect()
    }

    /// (位置描述, 文字, 是否允許「煞」)
    fn all_texts() -> Vec<(String, String, bool)> {
        let mut v = Vec::new();
        for c in CARDS {
            let k = c.key();
            v.push((format!("{k} fengshui"), c.fengshui.to_string(), true));
            v.push((format!("{k} research"), c.research.to_string(), false));
            if let Some(b) = c.badge {
                v.push((format!("{k} limitation"), b.limitation.to_string(), false));
            }
            for a in c.adjustments {
                v.push((format!("{k} adj {}", a.id), a.text.to_string(), false));
            }
        }
        for e in EVIDENCE {
            for (f, s) in [
                ("citation", e.citation),
                ("design", e.design),
                ("outcome", e.outcome),
                ("limitation", e.limitation),
            ] {
                v.push((format!("evidence {} {f}", e.key), s.to_string(), false));
            }
        }
        for (i, s) in fixed_texts().into_iter().enumerate() {
            v.push((format!("fixed #{i}"), s, false));
        }
        v
    }

    #[test]
    fn t3_no_banned_words_anywhere() {
        for (loc, text, _) in all_texts() {
            let hits = banned_hits(&text);
            assert!(hits.is_empty(), "{loc}: banned {hits:?} in 「{text}」");
        }
    }

    #[test]
    fn t3_sha_only_in_fengshui_column() {
        for (loc, text, sha_ok) in all_texts() {
            if !sha_ok {
                assert!(!text.contains('煞'), "{loc}: 「煞」outside 風水欄: {text}");
            }
        }
    }

    #[test]
    fn t3_allowlist_does_not_mask_real_hits() {
        assert!(banned_hits("研究不能證明這件事").is_empty());
        assert!(banned_hits("這不意外").is_empty());
        assert_eq!(banned_hits("研究證明了風水"), vec!["證明"]);
        assert_eq!(banned_hits("不能證明，但已證實"), vec!["證實"]);
        assert_eq!(banned_hits("小心意外"), vec!["意外"]);
    }

    #[test]
    fn t3_fengshui_column_uses_quoting_frame() {
        for c in CARDS {
            assert!(
                c.fengshui.starts_with("風水"),
                "{}: 風水欄須以引述句型開頭",
                c.key()
            );
        }
    }

    #[test]
    fn t3_light_items_disclose_question_differs() {
        for c in CARDS
            .iter()
            .filter(|c| c.item_id == "bed_light")
        {
            assert!(
                c.fengshui.contains("這裡問的是相近但不同的事"),
                "{}",
                c.key()
            );
        }
    }

    #[test]
    fn t3_no_research_cards_say_so_plainly() {
        for c in CARDS {
            if let Some(b) = c.badge {
                if b.reach == Reach::NoResearch {
                    assert!(
                        c.research.starts_with("目前沒有研究直接測過"),
                        "{}",
                        c.key()
                    );
                }
            }
        }
    }

    // ── T4 長度上限 ──

    #[test]
    fn t4_length_limits() {
        for c in CARDS {
            let k = c.key();
            let n = c.fengshui.chars().count();
            assert!(n <= 100, "{k} fengshui {n} > 100");
            let n = c.research.chars().count();
            assert!(n <= 140, "{k} research {n} > 140");
            for a in c.adjustments {
                let n = a.text.chars().count();
                assert!(n <= 60, "{k} adj {} {n} > 60", a.id);
            }
            if let Some(b) = c.badge {
                let n = b.limitation.chars().count();
                assert!(n <= 40, "{k} limitation {n} > 40");
            }
            assert!(!c.fengshui.is_empty() && !c.research.is_empty(), "{k}");
        }
    }

    // ── T6 徽章 ──

    /// 單卡徽章規則；回傳違規說明（供正反例共用）。
    fn badge_violation(c: &CardText) -> Option<String> {
        match (c.status.actionable(), c.badge) {
            (false, Some(_)) => return Some("ok card must not show a badge".into()),
            (true, None) => return Some("flag/mild card needs a badge".into()),
            _ => {}
        }
        for r in c.refs {
            if evidence_by_key(r).is_none() {
                return Some(format!("unknown evidence key {r}"));
            }
        }
        if let Some(b) = c.badge {
            if b.limitation.trim().is_empty() {
                return Some("empty limitation".into());
            }
            match b.reach {
                Reach::Direct => {
                    if c.refs.is_empty() {
                        return Some("direct without refs".into());
                    }
                    if let Some(r) = c
                        .refs
                        .iter()
                        .find(|r| !evidence_by_key(r).is_some_and(|e| e.full_text_read))
                    {
                        return Some(format!("direct cites {r} without full_text_read"));
                    }
                }
                Reach::Related => {
                    if c.refs.is_empty() {
                        return Some("related without refs".into());
                    }
                }
                Reach::NoResearch => {}
            }
        }
        None
    }

    #[test]
    fn t6_badges_valid_on_every_card() {
        for c in CARDS {
            if let Some(v) = badge_violation(c) {
                panic!("{}: {v}", c.key());
            }
        }
    }

    #[test]
    fn t6_direct_requires_full_text_read() {
        let mut c = *card_text("bed_back", Status::Flag, Some("window")).unwrap();
        c.badge = Some(Badge {
            reach: Reach::Direct,
            limitation: "測試",
        });
        // 帳本目前全為 false → 升 direct 必被擋
        assert!(badge_violation(&c).is_some());
        c.refs = &["no_such_paper"];
        assert!(badge_violation(&c).is_some());
    }

    #[test]
    fn t6_ok_card_with_badge_is_rejected() {
        let mut c = *card_text("bed_back", Status::Ok, None).unwrap();
        c.badge = related("x");
        assert!(badge_violation(&c).is_some());
    }

    #[test]
    fn t6_initial_reach_matches_spec_table() {
        use Reach::*;
        let expect: &[(&str, Option<&str>, Reach)] = &[
            ("bed_back", Some("window"), Related),
            ("bed_back", Some("passage"), Related),
            ("bed_back", Some("floating"), Related),
            ("bed_door_view", Some("hidden"), Related),
            ("bed_door_view", Some("facing"), NoResearch),
            ("bed_overhead", Some("beam"), NoResearch),
            ("bed_overhead", Some("heavy"), NoResearch),
            ("bed_mirror", None, NoResearch),
            ("bed_corner", None, Related),
            ("bed_door_door", None, NoResearch),
            ("bed_light", None, Related),
            ("bed_clutter", None, Related),
            ("seat_back", Some("window"), Related),
            ("seat_back", Some("passage"), Related),
            ("seat_door_view", Some("hidden"), Related),
            ("seat_door_view", Some("facing"), NoResearch),
            ("seat_overhead", None, NoResearch),
            ("seat_front", None, Related),
            ("seat_corner", None, Related),
            ("seat_light", None, Related),
            ("seat_clutter", None, Related),
        ];
        for (item, variant, reach) in expect {
            for c in CARDS
                .iter()
                .filter(|c| c.item_id == *item && c.variant == *variant && c.status.actionable())
            {
                assert_eq!(c.badge.map(|b| b.reach), Some(*reach), "{}", c.key());
            }
        }
        // spec §2.3：目前沒有任何一張卡達到 direct
        assert!(CARDS
            .iter()
            .all(|c| c.badge.map(|b| b.reach) != Some(Direct)));
    }

    // ── 固定文案形狀 ──

    #[test]
    fn fixed_copy_renders() {
        assert_eq!(
            reveal_miss_line(Reach::Direct, Reach::NoResearch),
            "你原本以為「有研究直接測過」，實際是「沒有人研究過」。"
        );
        assert_eq!(reveal_score_line(3, 5), "你猜對 3 / 5 張");
        assert_eq!(
            calibration_line(3, 5).as_deref(),
            Some("你的預測 5 次中有 3 次落在區間內")
        );
        assert_eq!(calibration_line(0, 0), None);
        assert_eq!(
            observation_note(Adherence::No, 2),
            "這段時間你沒有調整，分數還是變了 +2。這就是平常的起伏有多大。"
        );
        assert_eq!(observation_note(Adherence::Partial, 2), OBS_RESULT_NOTE);
        assert!(OPENING.contains("有些項目根本沒有人研究過"));
    }
}
