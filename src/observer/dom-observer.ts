/**
 * Generic DOM observer utility for video-fragments plugin.
 * Observes for elements matching a selector in all relevant documents and invokes a callback for each one exactly once.
 * Returns a cleanup function to disconnect all observers.
 */
export function observeElements(
  getAllRelevantDocuments: () => Document[],
  selector: string,
  onElement: (el: Element) => void
): () => void {
  const observers: MutationObserver[] = [];
  const seen = new WeakSet<Element>();

  const handleElement = (el: Element) => {
    if (!seen.has(el)) {
      seen.add(el);
      onElement(el);
    }
  };

  getAllRelevantDocuments().forEach(doc => {
    doc.querySelectorAll(selector).forEach(handleElement);
    const observer = new MutationObserver(mutations => {
      mutations.forEach(mutation => {
        if (mutation.type === 'childList') {
          mutation.addedNodes.forEach(node => {
            if (node instanceof Element) {
              if (node.matches(selector)) {
                handleElement(node);
              }
              node.querySelectorAll?.(selector)?.forEach?.(handleElement);
            }
          });
        }
      });
    });
    if (doc.body) observer.observe(doc.body, { childList: true, subtree: true });
    observers.push(observer);
  });

  return () => {
    observers.forEach(observer => observer.disconnect());
  };
}
