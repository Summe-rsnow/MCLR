<script setup lang="ts">
const PITCH = 136
const X0 = 248
const Y0 = 180
// [col, row] cells of the 4x5 pixel "R"; the accent block is the leg end.
const cells: Array<[number, number]> = [
  [0, 0], [1, 0], [2, 0],
  [0, 1], [3, 1],
  [0, 2], [1, 2], [2, 2],
  [0, 3], [2, 3],
  [0, 4],
]
const accent: [number, number] = [3, 4]
const pos = ([c, r]: [number, number]) => `translate(${X0 + c * PITCH} ${Y0 + r * PITCH})`
</script>

<template>
  <svg viewBox="0 0 1024 1024" xmlns="http://www.w3.org/2000/svg" aria-hidden="true">
    <defs>
      <clipPath id="mlogo-block"><rect width="120" height="120" rx="20" /></clipPath>
    </defs>
    <rect width="1024" height="1024" rx="224" style="fill: rgb(var(--c-primary, 59 130 246))" />
    <g v-for="cell in cells" :key="cell.join('-')" :transform="pos(cell)" clip-path="url(#mlogo-block)">
      <rect width="120" height="120" style="fill: rgb(var(--c-primary-on, 255 255 255))" />
      <rect y="96" width="120" height="24" fill="#0f172a" fill-opacity=".18" />
    </g>
    <g :transform="pos(accent)" clip-path="url(#mlogo-block)">
      <rect width="120" height="120" fill="#a3e635" />
      <rect y="96" width="120" height="24" fill="#0f172a" fill-opacity=".18" />
    </g>
  </svg>
</template>
