# ruruby benchmark results

## environment

Ruby version: 2.8.0  
CPU: Intel(R) Core(TM) i7-8700K CPU @ 3.70GHz  
OS: Ubuntu 18.04.4 LTS  

## execution time

|     benchmark     |     ruby      |     ruruby     |  rate  |
| :---------------: | :-----------: | :------------: | :----: |
| so_mandelbrot.rb  | 1.86 ± 0.06 s | 2.35 ± 0.02 s  | x 1.26 |
| app_mandelbrot.rb | 2.33 ± 0.02 s | 6.23 ± 0.09 s  | x 2.68 |
|      fibo.rb      | 0.49 ± 0.01 s | 1.96 ± 0.00 s  | x 3.98 |
|     block.rb      | 0.38 ± 0.00 s | 0.98 ± 0.06 s  | x 2.56 |
|    ao_bench.rb    | 9.55 ± 0.48 s | 26.91 ± 0.55 s | x 2.82 |

## optcarrot benchmark

|    benchmark    |       ruby        |      ruruby      |  rate  |
| :-------------: | :---------------: | :--------------: | :----: |
|    optcarrot    | 44.32 ± 0.57 fps  | 8.00 ± 0.28 fps  | x 5.54 |
| optcarrot --opt | 135.72 ± 0.84 fps | 36.04 ± 0.40 fps | x 3.77 |

## memory consumption

|     benchmark     | ruby  | ruruby |  rate   |
| :---------------: | :---: | :----: | :-----: |
| so_mandelbrot.rb  | 14.0K |  4.8K  | x 0.34  |
| app_mandelbrot.rb | 14.0K |  6.4K  | x 0.45  |
|      fibo.rb      | 13.8K |  4.6K  | x 0.34  |
|     block.rb      | 13.8K |  4.6K  | x 0.33  |
|    ao_bench.rb    | 14.5K |  7.2K  | x 0.49  |
|     optcarrot     | 68.5K | 81.3K  | x 1.19  |
|  optcarrot --opt  | 0.1M  |  1.5M  | x 17.31 |
