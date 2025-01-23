package main
import "base:runtime"
import "core:fmt"
import "core:mem"
import "core:strings"
import sql "odin-sqlite"

db_select_multi :: proc(
	cmd_end: string,
	array_arg: [dynamic]$T,
	args: ..any,
) -> (
	err: Result_Code,
) {
	b := strings.builder_make_len_cap(0, 128)
	defer strings.builder_destroy(&b)

	strings.write_string(&b, "SELECT ")

	ti := runtime.type_info_base(type_info_of(T))

	array_info := ti.variant.(runtime.Type_Info_Array)
	struct_info := ti.variant.(runtime.Type_Info_Struct)
	for name, i in struct_info.names[:struct_info.field_count] {
		strings.write_string(&b, name)

		if i != int(struct_info.field_count) - 1 {
			strings.write_byte(&b, ',')
		} else {
			strings.write_byte(&b, ' ')
		}
	}

	strings.write_string(&b, cmd_end)

	full_cmd := strings.to_string(b)
	// fmt.println(full_cmd)
	stmt := db_cache_prepare(full_cmd) or_return
	db_bind(stmt, ..args) or_return

	for {
		result := sql.step(stmt)

		if result == .DONE {
			break
		} else if result != .ROW {
			return result
		}

		// get column data per struct field
		for index in array_arg {
			for field in 0 ..< int(struct_info.field_count) {
				type := struct_info.types[field].id
				offset := struct_info.offsets[field]
				struct_value := any{rawptr(uintptr(index.data) + offset), type}
				db_any_column(stmt, i32(field), struct_value) or_return
			}
		}

	}

	return


}
