local M = {}

function M.on_tick(npc, events, context)
    for _, event in ipairs(events) do
        local kind = event:type()

        if kind == "entity_said" then
            local text = event:text()
            log("entity_said received: " .. text)
            if text == "test param" then
                local response = npc:get_param("response")
                npc:say(response)
            end
        end
    end
end

return M
