#!/usr/bin/env lua

local json = require("json")
local lfs = require("lfs")

-- Bookmark file path relative to HOME
local BOOKMARS_MAP = {
	brave = "Library/Application Support/BraveSoftware/Brave-Browser/Default/Bookmarks",
	brave_beta = "Library/Application Support/BraveSoftware/Brave-Browser-Beta/Default/Bookmarks",
	chrome = "Library/Application Support/Google/Chrome/Default/Bookmarks",
	chromium = "Library/Application Support/Chromium/Default/Bookmarks",
	opera = "Library/Application Support/com.operasoftware.Opera/Bookmarks",
	sidekick = "Library/Application Support/Sidekick/Default/Bookmarks",
	vivaldi = "Library/Application Support/Vivaldi/Default/Bookmarks",
	edge = "Library/Application Support/Microsoft Edge/Default/Bookmarks",
	arc = "Library/Application Support/Arc/User Data/Default/Bookmarks",
	safari = "Library/Safari/Bookmarks.plist",
}

-- Function to get environment variable as boolean
local function getEnvBool(name)
	local value = os.getenv(name)
	return value == "true" or value == "1"
end

-- Show favicon in results or default wf icon
local show_favicon = getEnvBool("show_favicon")

local BOOKMARKS = {}
-- Get Browser Histories to load based on user configuration
for k, v in pairs(BOOKMARS_MAP) do
	if getEnvBool(k) then
		table.insert(BOOKMARKS, v)
	end
end

-- Function to remove duplicates
local function removeDuplicates(li)
	local visited = {}
	local output = {}
	for _, entry in ipairs(li) do
		if not visited[entry[1]] then
			visited[entry[1]] = true
			table.insert(output, entry)
		end
	end
	return output
end

local function get_all_urls(the_json)
	local urls = {}

	local extract_data -- Forward Declaration

	local function get_container(input)
		if type(input) == "table" then
			for _, pair in pairs(input) do
				extract_data(pair)
			end
		end
	end

	-- Get url details
	function extract_data(data)
		if type(data) == "table" and data.type == "url" then
			table.insert(urls, { name = data.name, url = data.url })
		elseif type(data) == "table" and data.type == "folder" and #data.children > 0 then
			get_container(data.children) -- Now get_container is defined
		end
	end

	get_container(the_json)
	table.sort(urls, function(a, b)
		return a.name < b.name
	end)
	local final_list = {}
	for _, l in pairs(urls) do
		table.insert(final_list, { l.name, l.url })
	end
	return final_list
end

local function paths_to_bookmarks()
	local user_dir = os.getenv("HOME")
	local valid_bookmarks = {}
	for _, bookmark in ipairs(BOOKMARKS) do
		local full_path = user_dir .. "/" .. bookmark
		if lfs.attributes(full_path, "mode") == "file" then
			table.insert(valid_bookmarks, full_path)
			-- error
		else
			-- error
		end
	end
	return valid_bookmarks
end

local function get_json_from_file(file)
	local file_handle = io.open(file, "r")
	local content = file_handle:read("*all")
	-- error
	file_handle:close()
	return json.decode(content).roots
end

local function match(search_term, urls)
	local function is_in_url(tuple, st)
		for _, pair in ipairs(tuple) do
			if string.lower(tostring(pair)):find(string.lower(st)) then
				return true
			end
		end
		return false
	end

	local result = {}
	local search_terms = {}
	local search_operator = ""

	-- Determine search operator and split terms
	if search_term:find("&") then
		search_terms = search_term:split("&")
		search_operator = "&"
	elseif search_term:find("|") then
		search_terms = search_term:split("|")
		search_operator = "|"
	else
		search_terms = { search_term }
	end

	-- Provides operator logic
	local function matches(url)
		if search_operator == "&" then
			for _, term in ipairs(search_terms) do
				if not is_in_url(url, term) then
					return false
				end
			end
			return true
		elseif search_operator == "|" then
			for _, term in ipairs(search_terms) do
				if is_in_url(url, term) then
					return true
				end
			end
			return false
		else
			return is_in_url(url, table.concat(search_terms, " "))
		end
	end

	-- Process results
	for _, url in ipairs(urls) do
		if matches(url) then
			table.insert(result, url)
		end
	end

	return result
end

local function main()
	local query = table.concat(arg) or ""
	local bookmarks = paths_to_bookmarks()

	-- Retrives json from files in question
	-- Begins the process of interpreting that json
	local items = {}
	if #bookmarks > 0 then
		local matches = {}
		for _, bookmarks_file in ipairs(bookmarks) do
			if not bookmarks_file:find("Safari") then
				local bm_json = get_json_from_file(bookmarks_file)
				local bm = get_all_urls(bm_json)
				for _, item in ipairs(match(query, bm)) do
					table.insert(matches, item)
				end
			end
		end
		-- Takes the found urls for a search query and formats them to alfred schema
		for _, match in pairs(matches) do
			local name, url = match[1], match[2]
			table.insert(items, {
				title = name,
				subtitle = string.sub(url, 1, 80),
				arg = url,
				quicklookurl = url,
				mods = {
					cmd = {
						subtitle = "Other Actions...",
						arg = url,
					},
					alt = {
						subtitle = url,
						arg = url,
					},
				},
			})
		end
	end

	if #items == 0 then
		table.insert(items, {
			title = "No Bookmark found!",
			subtitle = string.format('Search "%s" in Google...', query),
			arg = string.format("https://www.google.com/search?q=%s", query),
		})
	end

	print(json.encode({ items = items }))
end

main()
