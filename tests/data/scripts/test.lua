local M = {}

function M.on_tick(npc, events)
    for _, event in ipairs(events) do
        local kind = event:type()

        if kind == "player_said" then
            local text = event:text()
            log("player_said received: " .. text)
            if text == "test query" then
                npc:say("test response")
            elseif text == "recall" then
                local remembered = npc:get_memory_str("remember", "<NO MEMORY>")
                npc:say("remembered: " .. remembered)
            elseif string.match(text, "^remember ") then
                local to_remember = string.gsub(text, "^remember ", "")
                npc:set_memory_str("remember", to_remember)
            end
        end
    end
end

return M
