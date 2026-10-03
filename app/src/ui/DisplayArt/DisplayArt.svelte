<!--
  DisplayArt: the display's background picture at its right edge: a dim violet glow over a dark
  sky with two hills, faded in from the ground on its left so the readouts over it stay
  brighter. Purely decorative, with no props; the theme picks its colours.
-->
<div class="art" aria-hidden="true">
  <div class="fade"></div>
</div>

<style>
  .art {
    position: relative;
    width: var(--display-art-width);
    height: var(--display-height);
    background:
      radial-gradient(
        circle at 62% 44%,
        var(--art-core) 0,
        var(--art-rim) 3%,
        var(--art-glow-20) 7%,
        var(--art-glow-6) 18%,
        var(--art-glow-0) 34%
      ),
      radial-gradient(
        ellipse 80% 44% at 40% 104%,
        var(--art-hill-near) 0,
        var(--art-hill-near) 58%,
        transparent 60%
      ),
      radial-gradient(ellipse 70% 38% at 92% 100%, var(--art-hill-far) 0, var(--art-hill-far) 62%, transparent 64%),
      linear-gradient(180deg, var(--art-sky-top) 0%, var(--art-sky-mid) 55%, var(--art-sky-foot) 100%);
  }
  .fade {
    position: absolute;
    inset: 0;
    background: linear-gradient(
      90deg,
      var(--art-fade-100) 0%,
      var(--art-fade-60) 22%,
      var(--art-fade-10) 60%,
      var(--art-fade-20) 100%
    );
  }
</style>
