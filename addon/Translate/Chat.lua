local addonName, ns = ...

-- Captured Chinese lines, keyed by a running id the companion echoes back.
ns.messages = {}
ns.translations = {}
local nextId = 1
local oldestId = 1
local KEEP = 300
local PREFIX = "TR#"

local CHAT_EVENTS = {
    "CHAT_MSG_SAY", "CHAT_MSG_YELL", "CHAT_MSG_WHISPER",
    "CHAT_MSG_PARTY", "CHAT_MSG_PARTY_LEADER",
    "CHAT_MSG_RAID", "CHAT_MSG_RAID_LEADER", "CHAT_MSG_RAID_WARNING",
    "CHAT_MSG_INSTANCE_CHAT", "CHAT_MSG_INSTANCE_CHAT_LEADER",
    "CHAT_MSG_GUILD", "CHAT_MSG_OFFICER", "CHAT_MSG_CHANNEL",
}

-- CJK Unified Ideographs (U+4E00..U+9FFF) start with UTF-8 lead bytes E4..E9.
local function HasChinese(text)
    return text:find("[\228-\233][\128-\191][\128-\191]") ~= nil
end

-- Hyperlinks and color codes would confuse the translator; send plain text.
local function StripCodes(text)
    text = text:gsub("|H.-|h(.-)|h", "%1")
    text = text:gsub("|c%x%x%x%x%x%x%x%x", "")
    text = text:gsub("|r", "")
    return text
end

function ns.CopyText(id)
    local entry = ns.messages[id]
    return entry and (PREFIX .. id .. " " .. entry.text) or ""
end

-- Chat frames each run the filter, so one line can arrive several times.
-- lineID is unique per line and maps it to a single stored id.
local byLine = {}

local function Store(text, author, lineID)
    local id = nextId
    nextId = nextId + 1
    ns.messages[id] = { text = StripCodes(text), author = author, lineID = lineID }
    if lineID then byLine[lineID] = id end
    while nextId - oldestId > KEEP do
        local old = ns.messages[oldestId]
        if old and old.lineID then byLine[old.lineID] = nil end
        ns.messages[oldestId] = nil
        ns.translations[oldestId] = nil
        oldestId = oldestId + 1
    end
    return id
end

function ns.ShowLatest()
    ns.ShowMessage(nextId - 1)
end

-- The companion replies with "TR#<id> <english>". Without a prefix the text
-- belongs to the line currently open in the window.
function ns.ReceiveTranslation(text, fallbackId)
    local id, english = text:match("^%s*" .. PREFIX .. "(%d+)%s+(.+)$")
    id = tonumber(id) or fallbackId
    english = english or text
    if not id or not ns.messages[id] then
        ns.Print("That translation's line is no longer in history.")
        return
    end
    ns.translations[id] = english
    local author = ns.messages[id].author or "?"
    DEFAULT_CHAT_FRAME:AddMessage("|cff66ccff[EN]|r " .. author .. ": " .. english)
    ns.RefreshResult(id)
end

local LABELS = {
    CHAT_MSG_SAY = "Say", CHAT_MSG_YELL = "Yell", CHAT_MSG_WHISPER = "Whisper",
    CHAT_MSG_PARTY = "Party", CHAT_MSG_PARTY_LEADER = "Party",
    CHAT_MSG_RAID = "Raid", CHAT_MSG_RAID_LEADER = "Raid", CHAT_MSG_RAID_WARNING = "Raid Warning",
    CHAT_MSG_INSTANCE_CHAT = "Instance", CHAT_MSG_INSTANCE_CHAT_LEADER = "Instance",
    CHAT_MSG_GUILD = "Guild", CHAT_MSG_OFFICER = "Officer",
}

-- Stores a Chinese line once and sends it to the pixel strip. Returns its id,
-- or nil when the line is not Chinese or cannot be read (secret values).
-- Arguments follow the CHAT_MSG_* payload; lineID is the 11th.
local function Capture(event, msg, author, _, channel, _, _, _, _, _, _, lineID)
    if ns.IsSecret(msg) or ns.IsSecret(author) or ns.IsSecret(channel) or ns.IsSecret(lineID) then
        return nil
    end
    if type(msg) ~= "string" or not HasChinese(msg) then return nil end
    if lineID and byLine[lineID] then return byLine[lineID] end

    local name = type(author) == "string" and Ambiguate(author, "short") or "?"
    local id = Store(msg, name, lineID)
    local label = LABELS[event]
    if event == "CHAT_MSG_CHANNEL" and type(channel) == "string" and channel ~= "" then
        label = channel
    end
    local speaker = label and ("[" .. label .. "] " .. name) or name
    ns.SendToStrip(id .. "\t" .. speaker .. "\t" .. ns.messages[id].text)
    return id
end

local function Filter(_, event, msg, author, ...)
    local id = Capture(event, msg, author, ...)
    if not id or not ns.db.showLinks then return false end
    local link = "|Haddon:" .. addonName .. ":" .. id .. "|h|cff66ccff[T]|r|h"
    return false, msg .. " " .. link, author, ...
end

local function OnLinkClicked(link)
    if type(link) ~= "string" then return end
    local name, id = link:match("^addon:([^:]+):(%d+)")
    if name == addonName then ns.ShowMessage(tonumber(id)) end
end

function ns.InitChat()
    -- Mainline moved some ChatFrame_ globals into ChatFrameUtil; probe both.
    local addFilter = ChatFrame_AddMessageEventFilter
    ns.filterSource = "ChatFrame_AddMessageEventFilter"
    if not addFilter and ChatFrameUtil and ChatFrameUtil.AddMessageEventFilter then
        addFilter = ChatFrameUtil.AddMessageEventFilter
        ns.filterSource = "ChatFrameUtil.AddMessageEventFilter"
    end
    if not addFilter then
        ns.filterSource = "none"
        ns.Print("No chat filter API found. Inline [T] links are off; the overlay still works.")
    else
        for _, event in ipairs(CHAT_EVENTS) do addFilter(event, Filter) end
    end

    -- Also capture lines on channels no chat window shows. Capture skips
    -- lines the filter already stored.
    for _, event in ipairs(CHAT_EVENTS) do
        ns.On(event, function(...) Capture(event, ...) end)
    end

    -- Clicks on |Haddon:...| links arrive through EventRegistry on Mainline.
    if EventRegistry and EventRegistry.RegisterCallback then
        EventRegistry:RegisterCallback("SetItemRef", function(_, link) OnLinkClicked(link) end, ns)
        ns.linkSource = "EventRegistry SetItemRef"
    else
        ns.linkSource = "none (use /tr)"
    end
end
