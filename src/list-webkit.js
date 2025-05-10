var appName = $params

var app    = Application(appName)
app.includeStandardAdditions = true
var items  = []
var titles = app.windows.tabs.name()
var urls   = app.windows.tabs.url()

for (var w = 0; w < titles.length; w++) {
  for (var t = 0; t < titles[w].length; t++) {
    items.push({
      title:       titles[w][t] || '',
      url:         urls[w][t]   || '',
      subtitle:    urls[w][t]   || '',
      windowIndex: w || '',
      tabIndex:    t || '',
      arg:         JSON.stringify([w, t])
    })
  }
}

JSON.stringify({ items })
