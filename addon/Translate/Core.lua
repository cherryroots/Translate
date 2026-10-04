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
    probe = function()
        local _, _, _, toc = GetBuildInfo()
        ns.Print("Interface " .. tostring(toc))
        ns.Print("LoggingChat: " .. tostring(LoggingChat ~= nil) .. (LoggingChat and (", on=" .. tostring(LoggingChat())) or ""))
        ns.Print("issecretvalue: " .. tostring(issecretvalue ~= nil))
        ns.Print("Chat filter API: " .. tostring(ns.filterSource))
        ns.Print("Link click hook: " .. tostring(ns.linkSource))
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
