  var browser = $params;
  var app     = Application(browser);
  app.includeStandardAdditions = true;

  // Walk every window → tab
  var items = [];
  var windows   = app.windows;
  for (var window_index = 0; window_index < windows.length; window_index++) {
    var win  = windows[window_index];
    var tabs = win.tabs || [];
    for (var tab_index = 0; tab_index < tabs.length; tab_index++) {
      var current_tab   = tabs[tab_index];

      var url   = current_tab.url()   || '';
      let matchUrl = url.replace(/(^\w+:|^)\/\//, "");
      let title = current_tab.title() || matchUrl;
      items.push({
        title:       title,
        subtitle:    url,
        url:         url,
        windowIndex: window_index,
        tabIndex:    tab_index,
        // Alfred’s “arg” must be a string
        arg:         JSON.stringify([window_index, tab_index])
      });
    }
  }

  return JSON.stringify({ items: items });
