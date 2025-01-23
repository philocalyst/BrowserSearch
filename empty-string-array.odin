package main

is_empty_string_array :: proc(input: []string) -> bool {
	for substring in input {
		if substring != "" {
			return false
		}
	}
	// No substrings were found to have content
	return true
}
