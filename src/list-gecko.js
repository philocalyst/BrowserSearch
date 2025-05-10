var appName  = $params

function notRunning() {
  return JSON.stringify({
    items: [
      {
        title:       `${appName} is not running`,
        subtitle:    `Press enter to launch ${appName}`,
        url:         '',
        windowIndex: 0,
        tabIndex:    0,
        arg:         appName
      }
    ]
  })
}

if (!Application(appName).running()) {
  return notRunning()
}

var se    = Application('System Events')
se.includeStandardAdditions = true
var procs = se.processes.whose({ name: appName }).get()

if (procs.length === 0) {
  return notRunning()
}

var proc  = procs[0]
var items = []

for (var w = 0; w < proc.windows.length; w++) {
  var win    = proc.windows[w]
  var groups = win.groups().get()
  var tabBar = groups.find(function(g) {
    return g.role() === 'AXTabGroup'
  })
  if (!tabBar) continue

  var buttons = tabBar.buttons().get()
  for (var t = 0; t < buttons.length; t++) {
    var uiTab = buttons[t]
    var title = ''
    try {
      title = uiTab.attributes.byName('AXTitle').value()
    } catch (e) {
      title = uiTab.name() || ''
    }
    items.push({
      title:       title,
      url:         '',
      subtitle:    title,
      windowIndex: w,
      tabIndex:    t,
      arg:         JSON.stringify([w, t])
    })
  }
}

JSON.stringify({ items })
