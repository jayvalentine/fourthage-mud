local M = {}

function M.on_tick(npc, events)
    for _, event in ipairs(events) do
        local kind = event:type()

        if kind == "player_said" then
            local text = event:text()
            if text == "test query" then
                npc:say("test response")
            end
        end
    end
end

return M
