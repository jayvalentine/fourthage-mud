local M = {}

function M.on_tick(npc, events)
    local current_location = npc:current_location()
    if current_location == "north_room" then
        local route = npc:get_route(current_location, "south_room")

        log("route: " .. table.concat(route, ", "))
        
        npc:set_memory_strs("route", route)
        npc:set_memory_int("route_index", 1)
    elseif current_location == "south_room" then
        local route = npc:get_route(current_location, "north_room")

        log("route: " .. table.concat(route, ", "))

        npc:set_memory_strs("route", route)
        npc:set_memory_int("route_index", 1)
    end
    
    local route_index = npc:get_memory_int("route_index", 1)
    local route = npc:get_memory_strs("route", {})

    if route_index <= #route then
        local direction_to_go = route[route_index]
        log("going " .. direction_to_go)
        npc:move(direction_to_go)
    
        route_index = route_index + 1
        npc:set_memory_int("route_index", route_index)
    end
end

return M
