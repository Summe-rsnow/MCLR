<script setup lang="ts">
import { computed } from 'vue'
import MarkdownIt from 'markdown-it'
import DOMPurify from 'dompurify'
import { openUrl } from '@tauri-apps/plugin-opener'

const props = withDefaults(
  defineProps<{
    source?: string | null
    format?: 'markdown' | 'html'
  }>(),
  {
    source: '',
    format: 'markdown',
  },
)

const md = new MarkdownIt({
  html: true,
  linkify: true,
  breaks: true,
})

if (typeof window !== 'undefined' && 'happyDOM' in window) {
  const doc = window.document
  doc.createNodeIterator = function (root: Node): NodeIterator {
    const nodes: Node[] = []
    const walk = (n: Node) => {
      nodes.push(n)
      for (const c of Array.from(n.childNodes || [])) {
        walk(c)
      }
    }
    walk(root)
    let idx = 0
    return {
      root,
      whatToShow: 0xffffffff,
      filter: null,
      pointerBeforeReferenceNode: true,
      referenceNode: root,
      nextNode() {
        return nodes[idx++] || null
      },
      previousNode() {
        return null
      },
      detach() {},
    } as NodeIterator
  }
  const nodeProto = Node.prototype as { nodeName?: string; nodeType: number; tagName?: string }
  const origDesc = Object.getOwnPropertyDescriptor(Node.prototype, 'nodeName')
  Object.defineProperty(nodeProto, 'nodeName', {
    get() {
      return this.nodeType === 3 ? '#text' : this.tagName || origDesc?.get?.call(this) || ''
    },
    configurable: true,
  })
}
const purify =
  typeof window !== 'undefined'
    ? DOMPurify(window)
    : (DOMPurify as unknown as ReturnType<typeof DOMPurify>)

if (purify && typeof purify.addHook === 'function') {
  purify.addHook('afterSanitizeAttributes', (n) => {
    if (n.tagName === 'A') {
      n.setAttribute('target', '_blank')
      n.setAttribute('rel', 'noopener noreferrer nofollow')
    }
    if (n.tagName === 'IMG') {
      n.setAttribute('loading', 'lazy')
      n.setAttribute('referrerpolicy', 'no-referrer')
      const src = n.getAttribute('src')
      if (src && src.startsWith('http://')) {
        n.setAttribute('src', src.replace(/^http:\/\//i, 'https://'))
      }
    }
  })
}

const CFG = {
  FORBID_TAGS: ['style', 'form', 'iframe', 'object', 'embed'],
  FORBID_ATTR: ['style'],
}

const html = computed(() => {
  const src = props.source || ''
  if (!src) return ''
  if (props.format === 'html') {
    return purify.sanitize(src, CFG)
  }
  return purify.sanitize(md.render(src), CFG)
})

async function onClick(e: MouseEvent) {
  const target = e.target as HTMLElement | null
  const a = target?.closest('a[href]') as HTMLAnchorElement | null
  if (!a) return
  const href = a.getAttribute('href') || ''
  if (href.startsWith('#')) {
    e.preventDefault()
    return
  }
  if (/^https?:\/\//i.test(href)) {
    e.preventDefault()
    try {
      await openUrl(href)
    } catch {
      window.open(href, '_blank')
    }
  }
}
</script>

<template>
  <div class="m-prose select-text break-words" v-html="html" @click="onClick" />
</template>

<style scoped>
.m-prose :deep(p),
.m-prose :deep(ul),
.m-prose :deep(ol),
.m-prose :deep(blockquote),
.m-prose :deep(pre),
.m-prose :deep(table),
.m-prose :deep(details) {
  margin: 0.5em 0;
}

.m-prose :deep(h1) {
  font-size: 1.5rem;
  font-weight: 700;
  margin: 1em 0 0.4em;
}

.m-prose :deep(h2) {
  font-size: 1.25rem;
  font-weight: 700;
  margin: 1em 0 0.4em;
}

.m-prose :deep(h3) {
  font-size: 1.1rem;
  font-weight: 700;
  margin: 1em 0 0.4em;
}

.m-prose :deep(h4),
.m-prose :deep(h5),
.m-prose :deep(h6) {
  font-size: 1rem;
  font-weight: 700;
  margin: 1em 0 0.4em;
}

.m-prose :deep(ul) {
  list-style: disc;
  padding-left: 1.4em;
}

.m-prose :deep(ol) {
  list-style: decimal;
  padding-left: 1.4em;
}

.m-prose :deep(a) {
  color: rgb(var(--c-primary));
  text-decoration: none;
}

.m-prose :deep(a:hover) {
  text-decoration: underline;
}

.m-prose :deep(img) {
  max-width: 100%;
  height: auto;
  display: inline-block;
  border-radius: 0.5rem;
  vertical-align: middle;
}

.m-prose :deep(center),
.m-prose :deep([align='center']) {
  text-align: center;
}

.m-prose :deep(hr) {
  border: 0;
  border-top: 1px solid rgb(var(--c-line));
  margin: 1em 0;
}

.m-prose :deep(code) {
  background: rgb(var(--c-sunken));
  padding: 0.1em 0.35em;
  border-radius: 0.375rem;
  font-size: 0.9em;
}

.m-prose :deep(pre) {
  background: rgb(var(--c-sunken));
  padding: 0.75em 1em;
  border-radius: 0.75rem;
  overflow-x: auto;
}

.m-prose :deep(pre code) {
  background: none;
  padding: 0;
}

.m-prose :deep(blockquote) {
  border-left: 3px solid rgb(var(--c-line));
  padding-left: 1em;
  color: rgb(var(--c-fg-muted));
}

.m-prose :deep(table) {
  border-collapse: collapse;
  display: block;
  overflow-x: auto;
}

.m-prose :deep(th),
.m-prose :deep(td) {
  border: 1px solid rgb(var(--c-line));
  padding: 0.35em 0.6em;
}

.m-prose :deep(summary) {
  cursor: pointer;
  font-weight: 600;
}

.m-prose :deep(kbd) {
  border: 1px solid rgb(var(--c-line));
  border-bottom-width: 2px;
  border-radius: 0.375rem;
  padding: 0 0.35em;
  font-size: 0.85em;
}
</style>
