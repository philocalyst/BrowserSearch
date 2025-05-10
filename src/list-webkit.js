var browser = $params

var app    = Application(browser)
app.includeStandardAdditions = true

// Walk through each
var items  = []
var windows = app.windows;

for (var window_index = 0; window_index < windows.length; window_index++) {

  var window = windows[window_index]
  var tabs = window.tabs || []; // Fallback if empty

  for (var tab_index = 0; tab_index < tabs.length; tab_index++) {
    var current_tab = tabs[tab_index];

    var url   = current_tab.url()   || '';
    let matchUrl = url.replace(/(^\w+:|^)\/\//, "");
    let title = current_tab.name() || matchUrl;
    items.push({
      title:       title || '',
      url:         url || '',
      subtitle:    url || '',
      windowIndex: window_index || '',
      tabIndex:    tab_index || '',
      arg:         JSON.stringify([window_index, tab_index])
    })
  }
}

JSON.stringify({ items })
