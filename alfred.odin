package main

import "core:encoding/json"
import "core:mem"
import "core:strings"

// Alfred Script Filter JSON Types
Arg :: union {
	string,
	[]string,
}

Icon :: struct {
	type: string `json:"type,omitempty"`,
	path: string `json:"path,omitempty"`,
}

ModifierAction :: struct {
	valid:     bool `json:"valid,omitempty"`,
	arg:       Arg `json:"arg,omitempty"`,
	subtitle:  string `json:"subtitle,omitempty"`,
	icon:      ^Icon `json:"icon,omitempty"`,
	variables: map[string]string `json:"variables,omitempty"`,
}

Mods :: struct {
	alt:   Maybe(ModifierAction) `json:"alt,omitempty"`,
	cmd:   Maybe(ModifierAction) `json:"cmd,omitempty"`,
	ctrl:  Maybe(ModifierAction) `json:"ctrl,omitempty"`,
	shift: Maybe(ModifierAction) `json:"shift,omitempty"`,
	fn:    Maybe(ModifierAction) `json:"fn,omitempty"`,
}

Text :: struct {
	copy:      string `json:"copy,omitempty"`,
	largetype: string `json:"largetype,omitempty"`,
}

AlfredItem :: struct {
	uid:          string `json:"uid,omitempty"`,
	title:        string `json:"title"`,
	subtitle:     string `json:"subtitle,omitempty"`,
	arg:          Maybe(Arg) `json:"arg,omitempty"`,
	autocomplete: string `json:"autocomplete,omitempty"`,
	icon:         Maybe(Icon) `json:"icon,omitempty"`,
	valid:        Maybe(bool) `json:"valid,omitempty"`,
	match:        string `json:"match,omitempty"`,
	type:         string `json:"type,omitempty"`,
	mods:         Maybe(Mods) `json:"mods,omitempty"`,
	text:         Maybe(Text) `json:"text,omitempty"`,
	quicklookurl: string `json:"quicklookurl,omitempty"`,
	variables:    map[string]string `json:"variables,omitempty"`,
}

Cache :: struct {
	seconds:     int `json:"seconds"`,
	loosereload: bool `json:"loosereload,omitempty"`,
}

AlfredOutput :: struct {
	items:         []AlfredItem `json:"items"`,
	variables:     map[string]string `json:"variables,omitempty"`,
	rerun:         Maybe(f64) `json:"rerun,omitempty"`,
	cache:         Maybe(Cache) `json:"cache,omitempty"`,
	skipknowledge: Maybe(bool) `json:"skipknowledge,omitempty"`,
}

// Custom marshaler for Arg union type
marshal_json :: proc(
	a: Arg,
	options: json.Marshal_Options,
	allocator: mem.Allocator,
) -> (
	[]u8,
	json.Marshal_Error,
) {
	switch v in a {
	case string:
		return json.marshal(v, options, allocator)
	case []string:
		return json.marshal(v, options, allocator)
	}
	return {}, {}
}

// Helper to generate JSON output
generate_alfred_json :: proc(
	items: []AlfredItem,
	variables: map[string]string = {},
	rerun: Maybe(f64) = nil,
	cache: Maybe(Cache) = nil,
	skipknowledge: Maybe(bool) = nil,
	allocator := context.allocator,
) -> (
	[]u8,
	json.Marshal_Error,
) {
	output := AlfredOutput {
		items         = items,
		variables     = variables,
		rerun         = rerun,
		cache         = cache,
		skipknowledge = skipknowledge,
	}

	options := json.Marshal_Options {
		spec = .JSON5,
	}

	return json.marshal(output, options, allocator)
}
