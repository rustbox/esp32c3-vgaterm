# Original

Hash `3ec5c707513c4b6a533ae8099fe242e81975947e`
===============================================

```
528 bytes
169776.02 draw cycles per byte
558 bytes
193957.2 draw cycles per byte
558 bytes
199648.33 draw cycles per byte
558 bytes
204402.34 draw cycles per byte
630 bytes
167890.98 draw cycles per byte
558 bytes
223444.6 draw cycles per byte
558 bytes
219381.02 draw cycles per byte
558 bytes
200000.14 draw cycles per byte
```

* Average bytes: `563`
* Average cycles: `197312.6`

# Directly set video memory

Hash: `f2627740c2a6f1ed37715a0d487b7f4987971bb9`
========================================

```
698 bytes
144043.72 draw cycles per byte
698 bytes
141167.67 draw cycles per byte
700 bytes
140613.3 draw cycles per byte
698 bytes
150651.81 draw cycles per byte
698 bytes
138351.31 draw cycles per byte
698 bytes
142081.36 draw cycles per byte
```

* Average bytes: `698`
* Average cycles: `142818.195`
<<<<<<< HEAD
<<<<<<< HEAD
=======
>>>>>>> 304c52f (characters contain whether they are dirty, chars are drawn per rect rather than pixel at a time, drawing comes after processing input characters)
* Improvement: `24%`

# Draw with `fill_contiguous`, buffer drawing characters, text carries dirty state

<<<<<<< HEAD
Hash: `304c52f8662db2f1363da8a9c36d6136a42d6236`
=======
Hash: `next`
>>>>>>> 304c52f (characters contain whether they are dirty, chars are drawn per rect rather than pixel at a time, drawing comes after processing input characters)
============================================

```
1562 bytes
12544.181 draw cycles per byte
2512 bytes
22627.264 draw cycles per byte
```

* Average bytes: `2037`
* Average cycles: `17585.7`
* Improvement: `3.6x`
<<<<<<< HEAD
=======
* Improvement: `24%`
>>>>>>> f262774 (setting the pixel color directly into the video buffer)
=======
>>>>>>> 304c52f (characters contain whether they are dirty, chars are drawn per rect rather than pixel at a time, drawing comes after processing input characters)
