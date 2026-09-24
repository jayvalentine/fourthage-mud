local M = {}

function M.on_tick(npc, events)
    local wait_time = npc:get_memory_int("wait_time", 0)
    if wait_time == 0 then
        local direction = npc:get_memory_str("direction", "north")
        if direction == "north" then
            npc:move("north")
            npc:set_memory_str("direction", "south")
        else
            npc:move("south")
            npc:set_memory_str("direction", "north")
        end
        npc:set_memory_int("wait_time", 2)
    else
        wait_time = wait_time - 1
        npc:set_memory_int("wait_time", wait_time)
    end
end

return M
