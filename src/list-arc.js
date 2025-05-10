ObjC.import('stdlib')
ObjC.import('Foundation')

var appName = $params

var app   = Application(appName)
app.includeStandardAdditions = true
var items = []

for (var w = 0; w < app.windows.length; w++) {
  var spaces = app.windows[w].spaces
  for (var s = 0; s < spaces.length; s++) {
    var tabs = spaces[s].tabs
    for (var t = 0; t < tabs.length; t++) {
      var tab = tabs[t]
      items.push({
        title:       tab.title()   || '',
        url:         tab.url()     || '',
        subtitle:    tab.url()     || '',
        windowIndex: w,
        tabIndex:    t,
        spaceIndex:  s,
        arg:         JSON.stringify([w, t, s])
      })
    }
  }
}

JSON.stringify({ items })
