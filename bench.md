# ruruby benchmark results

## environment

Ruby version: 2.8.0  
CPU: Intel(R) Core(TM) i7-8700K CPU @ 3.70GHz  
OS: Ubuntu 18.04.4 LTS  

## execution time

|     benchmark     |      ruby      |     ruruby     |  rate  |
| :---------------: | :------------: | :------------: | :----: |
| so_mandelbrot.rb  | 1.90 ± 0.02 s  | 2.56 ± 0.03 s  | x 1.34 |
| app_mandelbrot.rb | 2.35 ± 0.01 s  | 6.45 ± 0.04 s  | x 2.75 |
|      fibo.rb      | 0.47 ± 0.02 s  | 1.99 ± 0.03 s  | x 4.25 |
|     block.rb      | 0.38 ± 0.00 s  | 1.06 ± 0.02 s  | x 2.77 |
|    ao_bench.rb    | 10.05 ± 0.05 s | 26.94 ± 0.36 s | x 2.68 |

## optcarrot benchmark

|    benchmark    |       ruby        |      ruruby      |  rate  |
| :-------------: | :---------------: | :--------------: | :----: |
|    optcarrot    | 44.73 ± 0.41 fps  | 7.93 ± 0.07 fps  | x 5.64 |
| optcarrot --opt | 135.49 ± 1.11 fps | 35.28 ± 0.45 fps | x 3.84 |

## memory consumption

|     benchmark     | ruby  | ruruby |  rate   |
| :---------------: | :---: | :----: | :-----: |
| so_mandelbrot.rb  | 14.0K |  4.7K  | x 0.34  |
| app_mandelbrot.rb | 14.0K |  6.4K  | x 0.46  |
|      fibo.rb      | 13.8K |  4.7K  | x 0.34  |
|     block.rb      | 13.7K |  4.6K  | x 0.34  |
|    ao_bench.rb    | 14.5K |  7.2K  | x 0.49  |
|     optcarrot     | 68.4K | 81.2K  | x 1.19  |
|  optcarrot --opt  | 0.1M  |  1.5M  | x 17.72 |
