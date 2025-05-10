// Parse the arguments from the Rust call
const browser = $params[0];
const query = $params[1];

const { windowIndex, tabIndex } = getWindowAndTabIndex(query);
const { browserApp, browserWindow, maybeSystemWindow } = 
  getBrowserAndWindows(browser, windowIndex);

activateTab(browserApp, browserWindow, maybeSystemWindow, tabIndex);

return JSON.stringify({
  status: "success",
  browser: browser,
  windowIndex: windowIndex,
  tabIndex: tabIndex
});

function getWindowAndTabIndex(query) {
  const [windowIndex, tabIndex] = query.split(",").map(x => parseInt(x));
  return { windowIndex, tabIndex };
}

function getBrowserAndWindows(browserName, windowIndex) {
  const browserApp = Application(browserName);
  const browserWindow = browserApp.windows[windowIndex];
  let windowTitle = "";
  
  try {
    if (browserName.includes("Chrome") || browserName.includes("Brave") || browserName.includes("Chromium")) {
      windowTitle = browserWindow.activeTab.title();
    } else {
      windowTitle = browserWindow.title ? browserWindow.title() : "";
    }
  } catch (e) {
    windowTitle = "";
  }
  
  const maybeSystemWindow = getSystemWindow(browserName, windowIndex, windowTitle);
  return { browserApp, browserWindow, maybeSystemWindow };
}

function getSystemWindow(browserName, windowIndex, browserWindowTitle) {
  try {
    const se = Application("System Events");
    if (!se.processes[browserName].exists()) {
      return null;
    }
    
    const browserProcess = se.processes[browserName];
    const expectedTitlePrefix = browserWindowTitle ? `${browserWindowTitle} - ` : "";
    
    // Try by index first
    const systemWindow = browserProcess.windows[windowIndex];
    if (systemWindow && systemWindow.title && systemWindow.title().startsWith(expectedTitlePrefix)) {
      return systemWindow;
    }
    
    // If not found by index, try by title
    const allWindows = browserProcess.windows();
    for (let i = 0; i < allWindows.length; i++) {
      const win = allWindows[i];
      if (win.title && win.title().startsWith(expectedTitlePrefix)) {
        return win;
      }
    }
    
    return null;
  } catch (e) {
    return null;
  }
}

function activateTab(browser, browserWindow, maybeSystemWindow, tabIndex) {
  browserWindow.activeTabIndex = tabIndex + 1; // JXA uses 1-indexed tabs
  if (maybeSystemWindow) {
    maybeSystemWindow.actions["AXRaise"].perform();
  }
  browser.activate();
}
