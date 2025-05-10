const browser = $params[0];
const query = $params[1];


// In your working code, the query format for webkit was "windowIndex,url"
// Let's extract windowIndex but treat the tabIndex as a URL prefix to match
let [windowIndex, tabIdentifier] = query.split(",");
windowIndex = parseInt(windowIndex);

const app = Application(browser);
let window = app.windows[windowIndex];

try {
  // Find tab by URL or name if tabIdentifier seems to be a URL fragment
  if (tabIdentifier && tabIdentifier.includes("/")) {
    let foundTab = null;
    const tabs = window.tabs();

    
    for (let i = 0; i < tabs.length; i++) {
      const tab = tabs[i];
      const tabURL = tab.url ? tab.url() : tab.name();
      if (tabURL && tabURL.startsWith(tabIdentifier)) {
        foundTab = tab;
        break;
      }
    }
    
    if (foundTab) {

      app.activate();
      window.currentTab = foundTab;
      // Force tab window to front
      window.visible = false;
      window.visible = true;
      
      return JSON.stringify({
        status: "success",
        browser: browser,
        windowIndex: windowIndex,
        url: tabIdentifier
      });
    } else {
      return JSON.stringify({
        status: "error",
        message: "Tab with matching URL not found"
      });
    }
  } else {
    // If it's a standard index, use that
    const tabIndex = parseInt(tabIdentifier);
    window.currentTab = window.tabs[tabIndex];
    app.activate();
    
    return JSON.stringify({
      status: "success",
      browser: browser,
      windowIndex: windowIndex,
      tabIndex: tabIndex
    });
  }
} catch (e) {
  return JSON.stringify({
    status: "error",
    message: e.toString()
  });
}
