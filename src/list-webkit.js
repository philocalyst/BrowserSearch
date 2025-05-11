  let browser = $params;
  if (!Application(browser).running()) {
    return JSON.stringify({
      items: [
        {
          title: `${browser} is not running`,
          subtitle: `Press enter to launch ${browser}`,
        },
      ],
    });
  }

  let chrome = Application(browser);
  chrome.includeStandardAdditions = true;
  let windowCount = chrome.windows.length;
  let tabsTitle = chrome.windows.tabs.name();
  let tabsUrl = chrome.windows.tabs.url();
  let tabsMap = {};

  for (let window = 0; window < windowCount; window++) {
    for (let tab = 0; tab < tabsTitle[window].length; tab++) {
      let url = tabsUrl[window][tab] || "";
      let matchUrl = url.replace(/(^\w+:|^)\/\//, "");
      let title = tabsTitle[window][tab] || matchUrl;

      tabsMap[url] = {
        title,
        url,
        subtitle: url,
        windowIndex: window,
        tabIndex: tab,
        quicklookurl: url,
        arg: `${window},${url || title}`,
        match: `${title} ${decodeURIComponent(matchUrl).replace(
          /[^\w]/g,
          " ",
        )}`,
      };
    }
  }

  let items = Object.keys(tabsMap).reduce((acc, url) => {
    acc.push(tabsMap[url]);
    return acc;
  }, []);

  return JSON.stringify({ items });
