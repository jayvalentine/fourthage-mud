local M = {}

function M.on_tick(npc, events, context)
    local current_location = npc:current_location()
    if current_location == "north_room" then
        npc:set_memory("destination", "south_room")
    elseif current_location == "south_room" then
        npc:set_memory("destination", "north_room")
    end

    local dest = npc:get_memory("destination", "north_room")
    local route = context:get_route(current_location, dest)
    
    if #route > 0 then
        local direction_to_go = route[1]
        log("going " .. direction_to_go)
        npc:move(direction_to_go)
    end
end

return M
