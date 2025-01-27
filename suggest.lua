#!/usr/bin/env lua
local json = require("json")

local function concat(t1, t2)
	local result = {}
	for i = 1, #t1 do
		result[#result + 1] = t1[i]
	end
	for i = 1, #t2 do
		result[#result + 1] = t2[i]
	end
	return result
end

function alfred_format(itemNames)
	local items = {}
	for _, name in ipairs(itemNames) do
		table.insert(items, {
			uid = name,
			title = name,
			subtitle = 'Search "' .. name .. '" on Google',
			arg = name,
		})
	end
	return items
end

function main(search)
	-- local encodedQuery = http.encodeForQuery(arg[1])
	local queryURL = "https://duckduckgo.com/ac/?q=" .. search
	local body = io.popen("curl -s " .. queryURL):read("*all")
	local newResults = json.decode(body)
	local filteredResults = {}
	for _, result in pairs(newResults) do
		if result.phrase ~= arg[1] then
			table.insert(filteredResults, result.phrase)
		end
	end
	newResults = filteredResults

	-- Return final JSON
	return json.encode({
		skipknowledge = true,
		items = alfred_format(concat(arg, newResults)),
	})
end

print(main(table.concat(arg)))
