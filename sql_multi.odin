package main

import "base:runtime"
import "core:fmt"
import "core:mem"
import "core:reflect"
import "core:strings"
import sql "odin-sqlite"

select_multi :: proc(
	mod_statement: string,
	template: ^[dynamic]$T,
	database: []string = {},
) -> (
	err: Result_Code,
) {
	base_statement := strings.builder_make_len_cap(0, 256)
	defer strings.builder_destroy(&base_statement)

	clear(template)

	strings.write_string(&base_statement, "SELECT DISTINCT ")

	type_info := runtime.type_info_base(type_info_of(T))
	struct_info := type_info.variant.(runtime.Type_Info_Struct)
	empty_array := is_empty_string_array(database)

	//Iterates through each field and appends path to column to sql statement
	for name, index in struct_info.names[:struct_info.field_count] {
		// Provide support to multi-layered databases
		table_accessor: string
		if !empty_array {
			table_accessor = strings.concatenate({database[index], ".", name})
		} else {
			table_accessor = name
		}

		//Specific logic to cover for the diverging database naming schemes for both Firefox-based browsers and Safari-based Browsers
		if (name == "last_visit_time") {
			if (strings.contains(mod_statement, "moz_places")) {
				table_accessor = strings.concatenate({database[index], ".", "last_visit_date"})
			} else if (strings.contains(mod_statement, "safari")) {
				table_accessor = strings.concatenate({database[index], ".", "visit_time"})
			}
		}

		strings.write_string(&base_statement, table_accessor)

		if index != int(struct_info.field_count) - 1 {
			strings.write_byte(&base_statement, ',')
		} else {
			strings.write_byte(&base_statement, ' ')
		}
	}

	// Assign the mod statement to the base select
	strings.write_string(&base_statement, mod_statement)
	fmt.println(strings.to_string(base_statement))
	full_command := strings.to_string(base_statement)

	statement := db_cache_prepare(full_command) or_return

	db_bind(statement) or_return

	// Iterates over each row
	for {
		result := sql.step(statement)
		if result == .DONE {
			break
		} else if result != .ROW {
			return result
		}

		row: T
		for field in 0 ..< int(struct_info.field_count) {
			type := struct_info.types[field].id
			offset := struct_info.offsets[field]
			struct_value := any{rawptr(uintptr(&row) + offset), type}
			db_any_column(statement, i32(field), struct_value) or_return
		}
		append(template, row)
	}
	return
}
