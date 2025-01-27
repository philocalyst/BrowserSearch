package main
import "base:runtime"
import "core:c/libc"
import "core:encoding/cbor"
import "core:encoding/endian"
import "core:encoding/json"
import "core:fmt"
import "core:math"
import "core:os"
import "core:os/os2"
import "core:slice"
import "core:strconv"
import "core:strings"
import "core:time"
import "core:time/datetime"
import sql "odin-sqlite"

//Add support for slice encoding/decoding "slice.to_bytes/slice.reinterpret"
//TODO: Look into turning results into some form of a map for efficent diffing based on URL/ID

BROWSERS :: enum {
	brave,
	brave_Beta,
	chromium,
	chrome,
	opera,
	sidekick,
	vivaldi,
	edge,
	arc,
	safari,
	zen,
}

HISTORY_PATHS := []string {
	"/Library/Application Support/bravesoftware/brave-browser/default/history",
	"/Library/Application Support/bravesoftware/brave-browser-beta/default/history",
	"/Library/Application Support/chromium/default/history",
	"/Library/Application Support/google/chrome/default/history",
	"/Library/Application Support/com.operasoftware.opera/history",
	"/Library/Application Support/sidekick/default/history",
	"/Library/Application Support/vivaldi/default/history",
	"/Library/Application Support/microsoft edge/default/history",
	"/Library/Application Support/Arc/User Data/Default/History",
	"/Library/Safari/history.db",
	"/Library/Application Support/zen/Profiles/lm4bpegy.Default (release)/places.sqlite",
}

Result :: struct {
	id:              i32,
	title:           string,
	last_visit_time: i64,
	url:             string,
	visit_count:     i16,
}

OK_ARRAY := [5]string{}

SORT_RECENT := get_environment_boolean("sort_recent")
DATE_FORMAT := os.get_env("date_format")
FAVICON := get_environment_boolean("show_favicon")

get_environment_boolean :: proc(name: string) -> bool {
	value := os.get_env(name)
	return value == "true" || value == "1"
}

database_path_to_application :: proc(path: string) -> (application: string) {
	if (strings.contains(path, "zen")) {
		return "zen"
	} else if (strings.contains(path, "safari")) {
		return "safari"
	} else {
		return
	}
}

create_column_map :: proc(
	target_tables: ^map[string]string,
	application: string,
	template: ^[dynamic]$T,
) {
	type_info := runtime.type_info_base(type_info_of(T))
	struct_info := type_info.variant.(runtime.Type_Info_Struct)

	//Here we're iterating through each field (name) and creating a map of the exact column that is going to be extracted from in the sql statement, as each browser has a specific database design we're cooperating with.
	for name, index in struct_info.names[:struct_info.field_count] {
		if (application == "zen") {
			if (name == "last_visit_time") {
				target_tables[name] = strings.concatenate(
					{"history_visits", ".", "last_visit_date"},
				)
			} else {
				target_tables[name] = strings.concatenate({"moz_places", ".", name})
			}
		} else if (application == "safari") {
			if (name == "url" || name == "visit_count") {
				target_tables[name] = strings.concatenate({"history_items", ".", name})
			} else if (name == "title" || name == "id") {
				target_tables[name] = strings.concatenate({"history_visits", ".", name})
			} else if (name == "last_visit_time") {
				target_tables[name] = strings.concatenate({"history_visits", ".", "visit_time"})
			}
		} else {
			// Chromium-based
			target_tables[name] = strings.concatenate({"urls", ".", name})
		}
		OK_ARRAY[index] = target_tables[name]
	}
}

query_sqlite :: proc(
	database_path: string,
	query: string,
	template_struct: ^[dynamic]$T,
) -> ^[dynamic]Result {


	err := db_init(strings.clone_to_cstring(database_path))
	defer db_check(db_destroy())


	select_history(query, template_struct)

	return template_struct
}

select_history :: proc(sql_statement: string, template_array: ^[dynamic]$T) {

	type_info := runtime.type_info_base(type_info_of(T))
	struct_info := type_info.variant.(runtime.Type_Info_Struct)

	statement, err1 := db_cache_prepare(sql_statement)

	db_bind(statement)

	// Iterates over each row
	for {
		result := sql.step(statement)
		if result == .DONE {
			break
		} else if result != .ROW {
			return
		}

		row: T
		for field in 0 ..< int(struct_info.field_count) {
			type := struct_info.types[field].id
			offset := struct_info.offsets[field]
			struct_value := any{rawptr(uintptr(&row) + offset), type}
			db_any_column(statement, i32(field), struct_value)
		}
		append(template_array, row)
	}
	return
}

main :: proc() {
	using fmt

	user_directory := os.get_env("HOME")
	local_histories: [len(BROWSERS)]string = {}
	index := 0
	for browser in BROWSERS {
		value, _ := enum_value_to_string(browser)
		if get_environment_boolean(value) || value == "safari" || value == "arc" {
			local_histories[index] = strings.concatenate({user_directory, HISTORY_PATHS[browser]})
		}
		index += 1
	}
	// Add validation step here for error reporting

	data_location := create_storage_directory()
	cache_location := strings.concatenate({data_location, "browser-search.cbor"})
	time_location := strings.concatenate({data_location, "last-cached.txt"})


	results := make([dynamic]Result)
	defer write_to_cache(results, cache_location)
	defer delete(results)

	recent_results := make([dynamic]Result)
	defer delete(recent_results)

	load_cached_results(&results, cache_location)

	timer := new(time.Stopwatch)
	defer free(timer)
	time.stopwatch_start(timer)

	for application in BROWSERS {
		temporary_path: string
		application_path := local_histories[application]
		application_name, err := enum_value_to_string(application)
		if (application_path != "") {
			// Make replica of application sqlite
			temporary_path = create_temporary_database(application_path, application_name)

			target_tables := make_map(map[string]string)
			defer delete(target_tables)

			create_column_map(&target_tables, application_name, &results)

			select_statement := create_select_statement(application_name, target_tables)

			query_sqlite(temporary_path, select_statement, &recent_results)
		}
	}


	// Transform the results array into a map for accessing efficency
	url_map: map[string]^Result
	for &row in results {
		url_map[row.url] = &row
	}


	for &row in recent_results {
		// If this is a new url, make a new indice in the results array
		// Otherwise, update the outdated row
		if url_map[row.url] == {} {
			append(&results, row)
		} else {url_map[row.url] = &row}
	}

	items := make([dynamic]AlfredItem)

	for result in results {
		if strings.contains(result.title, os.args[1]) {
			append(
				&items,
				AlfredItem {
					title = result.title,
					subtitle = result.url,
					arg = result.url,
					valid = true,
				},
			)
		}
	}

	json_data, err := generate_alfred_json(items[:])
	println(string(json_data))

	write_last_cached_date(time_location)
}

in_array :: proc(array: []string, input: string) -> (res: bool) {
	for indice in array {
		if indice == input {
			return true
		}
	}
	return false
}

determine_join_clause :: proc(application: string) -> (clause: string) {
	if (application == "safari") {
		return " ON history_visits.history_item = history_items.id"
	} else if (application == "zen") {
		return ""
	} else {
		return ""
	}
}

create_select_statement :: proc(
	application: string,
	columns: map[string]string,
	args: ..any,
) -> (
	full_statement: string,
) {
	statement: strings.Builder
	strings.builder_init(&statement)
	defer strings.builder_destroy(&statement)

	last_cached_date, _ := os.read_entire_file_from_filename(
		"/Users/philocalyst/.local/share/browserSearch/last-cached.txt",
	)

	select_statement: strings.Builder
	strings.builder_init(&select_statement)
	defer strings.builder_destroy(&select_statement)

	from_statement: strings.Builder
	strings.builder_init(&from_statement)
	defer strings.builder_destroy(&from_statement)

	strings.write_string(&select_statement, "SELECT DISTINCT ")
	strings.write_string(&from_statement, "FROM ")

	base_tables := make([]string, 10)
	table_count := 0
	index := 0

	join_clause := determine_join_clause(application)

	for string, column in columns {
		up_one := strings.cut(column, 0, strings.index_any(column, "."))
		strings.write_string(&select_statement, OK_ARRAY[index])
		if column != "" && !in_array(base_tables, up_one) {
			if table_count >= 1 {
				strings.write_string(&from_statement, " INNER JOIN ")
			}
			strings.write_string(&from_statement, up_one)
			base_tables[table_count] = up_one
			table_count += 1
		}

		if (index != len(columns) - 1) {
			strings.write_string(&select_statement, ", ")
		} else {
			strings.write_string(&select_statement, " ")
		}
		index += 1
	}
	delete(base_tables)

	// Implement more programatic safari statements. The comma is to connect the statement types
	where_statement := fmt.tprintf(
		` WHERE %s IS NOT NULL AND %s != '' `,
		columns["title"],
		columns["title"],
	)

	last_cached_time, err1 := time.rfc3339_to_time_utc(string(last_cached_date))
	localized_time := localize_current_time(application, last_cached_time)

	and_statement := fmt.tprintf("AND %s >", columns["last_visit_time"])
	order_statement := fmt.tprintf(" ORDER BY %s DESC;", columns["last_visit_time"])

	database_name: string
	strings.write_string(&statement, strings.to_string(select_statement))
	strings.write_string(&statement, strings.to_string(from_statement))
	strings.write_string(&statement, join_clause)
	strings.write_string(&statement, where_statement)
	strings.write_string(&statement, and_statement)
	strings.write_i64(&statement, localized_time)
	strings.write_string(&statement, order_statement)

	return strings.clone(strings.to_string(statement))
}

load_cached_results :: proc(data: ^[dynamic]$T, location: string) {
	cbor_stream, _ := os.read_entire_file_from_filename(location)

	cbor.unmarshal(string(cbor_stream), data)
}

create_temporary_database :: proc(original_database: string, application: string) -> string {
	database_replica_path := strings.concatenate(
		{os.get_env("TMPDIR"), "browser-search-", application, ".sqlite"},
	)
	ok := os2.copy_file(database_replica_path, original_database)

	return database_replica_path
}

create_storage_directory :: proc() -> (filename: string) {
	data_home := os.get_env("XDG_DATA_HOME")
	if (data_home == "") {
		data_home = strings.concatenate({os.get_env("HOME"), "/.local/share/"})
	}
	local_location: strings.Builder
	strings.builder_init(&local_location)

	strings.write_string(&local_location, data_home)
	strings.write_string(&local_location, "browserSearch/")

	//This is running when local_location is just the folder, so it creates it at that location
	if !os.exists(strings.to_string(local_location)) {
		err := os.make_directory(strings.to_string(local_location), 0o666)
		fmt.println("directory_make", strings.to_string(local_location))
	}

	return strings.to_string(local_location)
}

write_to_cache :: proc(data: [dynamic]Result, location: string) {
	// Marshall here into cbor for inexpensive cold start
	cbor_stream, err := read_and_marshal(data)
	ok := os.write_entire_file(location, cbor_stream, false)
}

digit_count :: proc(number: i64) -> i64 {
	if number == 0 {
		return 1
	}
	count := 0
	stunt_double := number
	for stunt_double != 0 {
		stunt_double = stunt_double / 10
		count += 1
	}
	return i64(math.pow10_f64(f64(count - 1)))
}

localize_current_time :: proc(application: string, current_time: time.Time) -> (adjusted: i64) {
	if (application == "safari") {
		core_data_epoch, err2 := datetime.components_to_datetime(2001, 1, 1, 0, 0, 0)
		current_datetime, err1 := time.time_to_datetime(current_time)
		delta, err3 := datetime.subtract_datetimes(current_datetime, core_data_epoch)
		time_in_seconds := (delta.days * time.SECONDS_PER_DAY) + (delta.seconds)
		return time_in_seconds
	} else {
		// Using windows epoch here
		windows_epoch, err2 := datetime.components_to_datetime(1601, 1, 1, 0, 0, 0)
		current_datetime, err1 := time.time_to_datetime(current_time)
		delta, err3 := datetime.subtract_datetimes(current_datetime, windows_epoch)
		return ((delta.days * time.SECONDS_PER_DAY) + (delta.seconds)) * 1000000
	}
}

write_last_cached_date :: proc(location: string) {
	if !os.exists(location) {
		resulting_handle, err := os.open(location, os.O_CREATE, 0o666)
		defer os.close(resulting_handle)
	}
	date_string, _ := time.time_to_rfc3339(time.now())
	os.write_entire_file(location, transmute([]u8)fmt.tprintf("%v", date_string))
}

read_and_marshal :: proc(data: [dynamic]Result) -> ([]byte, cbor.Marshal_Error) {
	cbor_stream, marshal_ok := cbor.marshal_into_bytes(data)
	return cbor_stream, marshal_ok
}
