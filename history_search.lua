#!/usr/bin/env lua

local lfs = require("lfs")
local sqlite3 = require("lsqlite3")
local json = require("json")

-- History map
local HISTORY_MAP = {
	brave = "Library/Application Support/BraveSoftware/Brave-Browser/Default/History",
	brave_beta = "Library/Application Support/BraveSoftware/Brave-Browser-Beta/Default/History",
	chromium = "Library/Application Support/Chromium/Default/History",
	chrome = "Library/Application Support/Google/Chrome/Default/History",
	opera = "Library/Application Support/com.operasoftware.Opera/History",
	sidekick = "Library/Application Support/Sidekick/Default/History",
	vivaldi = "Library/Application Support/Vivaldi/Default/History",
	edge = "Library/Application Support/Microsoft Edge/Default/History",
	arc = "Library/Application Support/Arc/User Data/Default/History",
	safari = "Library/Safari/History.db",
}

-- Function to get environment variable as boolean
local function getEnvBool(name)
	local value = os.getenv(name)
	return value == "true" or value == "1"
end

-- Get Browser Histories to load per env (true/false)
local HISTORIES = {}
for k, v in pairs(HISTORY_MAP) do
	if getEnvBool(k) or k == "arc" then
		table.insert(HISTORIES, v)
	end
end

-- Get ignored Domains settings
local ignored_domains = {}
local d = os.getenv("ignored_domains")
if d then
	for domain in d:gmatch("([^,]+)") do
		table.insert(ignored_domains, domain)
	end
end

-- Show favicon in results or default wf icon
local show_favicon = getEnvBool("show_favicon")

-- if set to true history entries will be sorted
-- based on recent visitied otherwise number of visits
local sort_recent = getEnvBool("sort_recent")

-- Date format settings
local DATE_FMT = os.getenv("date_format") or "%d. %B %Y"

-- Function to get valid paths to history from HISTORIES variable
local function history_paths()
	local user_dir = os.getenv("HOME")
	local valid_hists = {}
	for _, h in ipairs(HISTORIES) do
		local full_path = user_dir .. "/" .. h
		if lfs.attributes(full_path, "mode") == "file" then
			table.insert(valid_hists, full_path)
		else
			io.stderr:write(full_path .. " → NOT found")
		end
	end
	return valid_hists
end

-- Function to execute SQL query on history database
local function sql(db)
	local res = {}
	local temp_db = os.tmpname()
	os.execute(string.format("cp %s %s", string.format("%q", db), string.format("%q", temp_db)))

	local conn = sqlite3.open(temp_db)
	local select_statement
	if db:find("Safari") then
		-- Gets url, title, visit count, and time (translated to unix epoch from Mac's CoreData timestamp)
		select_statement = [[
        SELECT history_items.url, history_visits.title, history_items.visit_count,
               (history_visits.visit_time + 978307200) AS last_visit_time
        FROM history_items
            INNER JOIN history_visits
            ON history_visits.history_item = history_items.id
        WHERE history_items.url IS NOT NULL AND
            history_visits.title IS NOT NULL AND
            history_items.url != ''
        ORDER BY visit_count DESC
    ]]
	else
		select_statement = [[
        SELECT DISTINCT urls.url, urls.title, urls.visit_count,
               (urls.last_visit_time/1000000 + strftime('%s', '1601-01-01')) AS last_visit_time
        FROM urls, visits
        WHERE urls.id = visits.url AND
        urls.title IS NOT NULL AND
        urls.title != ''
        ORDER BY last_visit_time DESC
    ]]
	end

	for row in conn:nrows(select_statement) do
		table.insert(res, { row.url, row.last_visit_time, row.title, row.visit_count })
	end

	conn:close()
	os.remove(temp_db)
	return res
end

function printTable(t, indent)
	indent = indent or 0
	for k, v in pairs(t) do
		local formatting = string.rep("  ", indent) .. k .. ": "
		if type(v) == "table" then
			print(formatting)
			printTable(v, indent + 1)
		else
			print(formatting .. tostring(v))
		end
	end
end

-- Function to search in tuples
local function search_in_tuples(tuples, search)
	local function is_in_tuple(tple, st)
		for _, e in ipairs(tple) do
			if string.lower(tostring(e)):find(string.lower(st)) then
				return true
			end
		end
		return false
	end

	local search_terms = {}
	for term in search:gmatch("([^&|]+)") do
		table.insert(search_terms, term)
	end

	local result = {}
	for _, t in ipairs(tuples) do
		local match = true
		for _, term in ipairs(search_terms) do
			if not is_in_tuple(t, term) then
				match = false
				break
			end
		end
		if match then
			table.insert(result, t)
		end
	end
	return result
end

-- Function to remove duplicates
local function removeDuplicates(list)
	local visited = {}
	local output = {}
	for _, entry in ipairs(list) do
		if not visited[entry[3]] and not visited[entry[1]] then
			visited[entry[1]] = true
			visited[entry[3]] = true
			table.insert(output, entry)
		end
	end
	return output
end

-- Function to remove ignored domains
local function remove_ignored_domains(results, ignored_domains)
	local new_results = {}
	for _, r in ipairs(results) do
		local ignore = false
		for _, domain in ipairs(ignored_domains) do
			if r[1]:find(domain) then
				ignore = true
				break
			end
		end
		if not ignore then
			table.insert(new_results, r)
		end
	end
	return new_results
end

-- Main function
local function main()
	local search_term = arg[1]
	local locked_history_dbs = history_paths()

	if #locked_history_dbs == 0 then
		print(json.encode({
			items = {
				{
					title = "Browser History not found!",
					subtitle = "Ensure Browser is installed or choose available browser(s) in CONFIGURE WORKFLOW",
					valid = false,
				},
			},
		}))
		return
	end

	local results = {}
	if search_term then
		for _, db in ipairs(locked_history_dbs) do
			local db_results = sql(db)
			for _, result in ipairs(db_results) do
				table.insert(results, result)
			end
		end
		results = search_in_tuples(results, search_term)
		results = removeDuplicates(results)
		if #ignored_domains > 0 then
			results = remove_ignored_domains(results, ignored_domains)
		end
		if #results > 30 then
			local limited_results = {}
			for i = 1, 30 do
				limited_results[i] = results[i]
			end
			results = limited_results
		end
	else
		return
	end

	local items = {}
	if #results > 0 then
		for _, result in ipairs(results) do
			local url, last_visit, title, visits = result[1], result[2], result[3], result[4]
			table.insert(items, {
				title = title,
				subtitle = string.format("Last visit: %s (Visits: %d)", os.date("%x", last_visit), tonumber(visits)),
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
	else
		table.insert(items, {
			title = "Nothing found in History!",
			subtitle = string.format('Search "%s" in Google?', search_term),
			arg = string.format("https://www.google.com/search?q=%s", search_term),
		})
	end
	print(json.encode({ items = items }))
end

main()
