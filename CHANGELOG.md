<a name="0.5.2"></a>
## 0.5.2 (2020-07-07)


#### Features

* **mutations:**  add support for Box<T> ([d6a2cc76](https://github.com/microsoft/lain/commit/d6a2cc76a3bffdb9719c9a385be33f4e29f70769))



<a name="0.5.1"></a>
## 0.5.1 (2020-06-18)


#### Bug Fixes

* **mutations:**
  *  change how/when fixups are performed ([f51644e1](https://github.com/microsoft/lain/commit/f51644e1bd86f46586a7b24e26183082f6b33a94))
  *  revert 1d79b7e0 which inlined generated mutations ([bf7ed1e7](https://github.com/microsoft/lain/commit/bf7ed1e77de5f29bc37ac06a2016b01734478545))
* **serialization:**  revert 2076d121 which inlined generated serde code ([917c1baa](https://github.com/microsoft/lain/commit/917c1baa7d4a54c5faf404542e7ac3609741ebba))



<a name="0.5"></a>
## 0.5 (2020-06-16)


#### Bug Fixes

* **constraints:**  Mutating now strictly respects max size constraint ([2e487784](https://github.com/microsoft/lain/commit/2e4877845d9a02df736944dbe09e56f3daf86cab))
* **mutations:**
  *  ignore_chance should now be working for enum variants ([a97cb173](https://github.com/microsoft/lain/commit/a97cb173f4b20f6f1923279d007a9fe9538c94fb))
  *  partially fix regression where ignore_chance did not work for enum variants ([e8d23ae2](https://github.com/microsoft/lain/commit/e8d23ae2768292a89e015b3faf73eee4bc9ca3af))

#### Performance

* **mutations:**  hint to inline struct fuzzing functions ([1d79b7e0](https://github.com/microsoft/lain/commit/1d79b7e0732f6a3ef39e302e50fd5f79640cd143))
* **serialization:**  inline generated serialization code ([2076d121](https://github.com/microsoft/lain/commit/2076d121caaab15300163e284a706a55b42553c6))



