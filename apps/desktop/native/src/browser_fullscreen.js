// Phone previews fill their child viewport without fullscreening the host window.
(() => {
  let current = null;
  let restore = [];
  let observer;
  const change = element => {
    // Native fullscreen resolves its promise before delivering change events.
    // Players set their requested state during that promise continuation.
    setTimeout(() => {
      const target = element.isConnected ? element : document;
      target.dispatchEvent(new Event('fullscreenchange', { bubbles: true }));
      target.dispatchEvent(new Event('webkitfullscreenchange', { bubbles: true }));
    }, 0);
  };
  const set = (element, property, value) => {
    const previous = element.style.getPropertyValue(property);
    const priority = element.style.getPropertyPriority(property);
    restore.push(() => previous ? element.style.setProperty(property, previous, priority) : element.style.removeProperty(property));
    element.style.setProperty(property, value, 'important');
  };
  const exit = () => {
    if (!current) return Promise.resolve();
    const previous = current;
    current = null;
    observer?.disconnect();
    for (const reset of restore.reverse()) reset();
    restore = [];
    change(previous.isConnected ? previous : document);
    return Promise.resolve();
  };
  const enter = function () {
    if (!this.isConnected) return Promise.reject(new TypeError('Fullscreen target is detached'));
    if (current === this) return Promise.resolve();
    if (current) exit();
    current = this;
    // Ancestor transforms/containment must not constrain the fixed player.
    for (let parent = this.parentElement; parent; parent = parent.parentElement) {
      for (const property of ['transform', 'filter', 'perspective', 'contain']) set(parent, property, 'none');
      set(parent, 'overflow', 'visible');
    }
    set(document.documentElement, 'overflow', 'hidden');
    for (const [property, value] of Object.entries({ position: 'fixed', inset: '0', width: '100vw', height: '100vh', 'max-width': 'none', 'max-height': 'none', margin: '0', 'z-index': '2147483647', background: 'black' })) set(this, property, value);
    observer = new MutationObserver(() => { if (current && !current.isConnected) exit(); });
    observer.observe(document, { childList: true, subtree: true });
    change(this);
    return Promise.resolve();
  };
  for (const name of ['requestFullscreen', 'webkitRequestFullscreen', 'webkitRequestFullScreen']) Object.defineProperty(Element.prototype, name, { configurable: true, value: enter });
  for (const name of ['exitFullscreen', 'webkitExitFullscreen', 'webkitCancelFullScreen']) Object.defineProperty(Document.prototype, name, { configurable: true, value: exit });
  for (const name of ['fullscreenElement', 'webkitFullscreenElement', 'webkitCurrentFullScreenElement']) Object.defineProperty(Document.prototype, name, { configurable: true, get: () => current });
  for (const name of ['fullscreenEnabled', 'webkitFullscreenEnabled']) Object.defineProperty(Document.prototype, name, { configurable: true, get: () => true });
  Object.defineProperty(Document.prototype, 'webkitIsFullScreen', { configurable: true, get: () => !!current });
  Object.defineProperty(Document.prototype, 'fullscreen', { configurable: true, get: () => !!current });
  // Mobile players compare fullscreen window/screen areas to detect backgrounding.
  // During contained fullscreen both areas refer to this child viewport.
  for (const [object, prototype, property, dimension] of [
    [window, Window.prototype, 'outerWidth', 'innerWidth'],
    [window, Window.prototype, 'outerHeight', 'innerHeight'],
    [screen, Screen.prototype, 'width', 'innerWidth'],
    [screen, Screen.prototype, 'height', 'innerHeight'],
  ]) {
    const descriptor = Object.getOwnPropertyDescriptor(object, property) ?? Object.getOwnPropertyDescriptor(prototype, property);
    const original = object[property];
    Object.defineProperty(object, property, { configurable: true, get: () => current ? window[dimension] : descriptor?.get ? descriptor.get.call(object) : original });
  }
  document.addEventListener('keydown', event => {
    if (event.key === 'Escape' && current) { event.preventDefault(); event.stopImmediatePropagation(); exit(); }
  }, true);
  window.addEventListener('pagehide', exit);
})();
