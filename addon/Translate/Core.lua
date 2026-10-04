local addonName, ns = ...

-- One event frame, one handler table keyed by event name.
ns.handlers = {}
local frame = CreateFrame("Frame")
ns.eventFrame = frame

function ns.On(event, fn)
    ns.handlers[event] = fn
    frame:RegisterEvent(event)
end

frame:SetScript("OnEvent", function(_, event, ...)
    local fn = ns.handlers[event]
    if fn then fn(...) end
end)

local DEFAULTS = {
    chatLog = true,   -- turn on WoW's chat log so the companion can read it
    showLinks = true, -- add a clickable [T] after Chinese chat lines
    flushLog = true,  -- reopen the chat log after chat so WoW writes it to disk
}

function ns.Print(msg)
    DEFAULT_CHAT_FRAME:AddMessage("|cff66ccffTranslate:|r " .. msg)
end

-- Secret values (Midnight) cannot be compared or concatenated.
function ns.IsSecret(value)
    return issecretvalue ~= nil and issecretvalue(value) == true
end

function ns.SetChatLog(enabled)
    if not LoggingChat then
        ns.Print("LoggingChat is missing on this client. Type /chatlog to start the chat log by hand.")
        return
    end
    if enabled and not LoggingChat() then
        LoggingChat(true)
        ns.Print("Chat log on. The companion app reads it from the Logs folder.")
    elseif not enabled and LoggingChat() then
        LoggingChat(false)
        ns.Print("Chat log off.")
    end
end

-- WoW buffers the chat log and may only write it out when the file closes.
-- Turning logging off and on again closes and reopens it. Debounced so busy
-- chat reopens the file at most once per FLUSH_DELAY.
local FLUSH_DELAY = 0.5
local flushPending = false

function ns.FlushLog()
    if LoggingChat and LoggingChat() then
        LoggingChat(false)
        LoggingChat(true)
    end
end

function ns.ScheduleFlush()
    if flushPending or not ns.db.flushLog or not C_Timer then return end
    flushPending = true
    C_Timer.After(FLUSH_DELAY, function()
        flushPending = false
        ns.FlushLog()
    end)
end

ns.On("ADDON_LOADED", function(name)
    if name ~= addonName then return end
    TranslateDB = TranslateDB or {}
    for k, v in pairs(DEFAULTS) do
        if TranslateDB[k] == nil then TranslateDB[k] = v end
    end
    ns.db = TranslateDB
    frame:UnregisterEvent("ADDON_LOADED")
end)

ns.On("PLAYER_LOGIN", function()
    ns.SetChatLog(ns.db.chatLog)
    if ns.InitChat then ns.InitChat() end
end)

local function ShowHelp()
    ns.Print("/tr  open the translate window on the latest Chinese line")
    ns.Print("/tr log  toggle the chat log (the companion needs it on)")
    ns.Print("/tr links  toggle the [T] links after Chinese lines")
    ns.Print("/tr test  say a Chinese test line so it reaches the chat log")
    ns.Print("/tr flush  toggle forced chat log writes (on by default)")
    ns.Print("/tr probe  report which client features this addon found")
end

local commands = {
    [""] = function() ns.ShowLatest() end,
    log = function()
        ns.db.chatLog = not ns.db.chatLog
        ns.SetChatLog(ns.db.chatLog)
    end,
    links = function()
        ns.db.showLinks = not ns.db.showLinks
        ns.Print("[T] links " .. (ns.db.showLinks and "on." or "off."))
    end,
    flush = function()
        ns.db.flushLog = not ns.db.flushLog
        ns.Print("Forced chat log writes " .. (ns.db.flushLog and "on." or "off."))
    end,
    test = function()
        -- Mainline moved SendChatMessage into C_ChatInfo; probe both.
        local send = (C_ChatInfo and C_ChatInfo.SendChatMessage) or SendChatMessage
        if not send then
            ns.Print("No SendChatMessage on this client. Type /s 需要奶妈 by hand.")
            return
        end
        send("需要奶妈 来人 (Translate test)", "SAY")
        ns.Print("Sent a test line in /say. The overlay should show it in English shortly.")
    end,
    probe = function()
        local _, _, _, toc = GetBuildInfo()
        ns.Print("Interface " .. tostring(toc))
        ns.Print("LoggingChat: " .. tostring(LoggingChat ~= nil) .. (LoggingChat and (", on=" .. tostring(LoggingChat())) or ""))
        ns.Print("issecretvalue: " .. tostring(issecretvalue ~= nil))
        ns.Print("Chat filter API: " .. tostring(ns.filterSource))
        ns.Print("Link click hook: " .. tostring(ns.linkSource))
        ns.Print("C_Timer: " .. tostring(C_Timer ~= nil) .. ", forced log writes: " .. tostring(ns.db.flushLog))
        ns.Print("SendChatMessage: " .. ((C_ChatInfo and C_ChatInfo.SendChatMessage) and "C_ChatInfo" or (SendChatMessage and "global" or "none")))
    end,
    help = ShowHelp,
}

SLASH_TRANSLATE1 = "/tr"
SLASH_TRANSLATE2 = "/translate"
SlashCmdList.TRANSLATE = function(input)
    local cmd = strlower(strtrim(input or ""))
    local fn = commands[cmd] or ShowHelp
    fn()
end
