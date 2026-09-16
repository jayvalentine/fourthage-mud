local M = {}

function M.on_tick(npc, events)
    for _, event in ipairs(events) do
        local kind = event:type()

        if kind == "player_said" then
            local text = string.lower(event:text())

            if string.find(text, "woof") or string.find(text, "bark") then
                npc:emote("tilts its head in confusion.")
            elseif string.find(text, "food") or string.find(text, "treat") then
                npc:emote("wags its tail excitedly!")
            else
                npc:say("Woof!")
            end
        end
    end
end

return M
