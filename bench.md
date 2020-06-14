# ruruby benchmark results

## environment

Ruby version: 2.8.0  
CPU: Intel(R) Core(TM) i7-8700K CPU @ 3.70GHz  
OS: Ubuntu 18.04.4 LTS  

## execution time

|     benchmark     |       ruby       |     ruruby      |  rate  |
| :---------------: | :--------------: | :-------------: | :----: |
| so_mandelbrot.rb  |  1.72 ± 0.02 s   |  2.52 ± 0.05 s  | x 1.46 |
| app_mandelbrot.rb |  2.18 ± 0.02 s   |  5.97 ± 0.10 s  | x 2.74 |
|      fibo.rb      |  0.45 ± 0.01 s   |  1.92 ± 0.03 s  | x 4.27 |
|     block.rb      |  0.36 ± 0.01 s   |  1.00 ± 0.03 s  | x 2.76 |
|    ao_bench.rb    |  8.94 ± 0.05 s   | 24.03 ± 0.18 s  | x 2.69 |
|     optcarrot     | 48.46 ± 1.33 fps | 8.39 ± 0.55 fps | x 5.78 |

## memory consumption

|     benchmark     | ruby  | ruruby |  rate  |
| :---------------: | :---: | :----: | :----: |
| so_mandelbrot.rb  | 14.0K |  4.9K  | x 0.35 |
| app_mandelbrot.rb | 14.0K | 17.4K  | x 1.24 |
|      fibo.rb      | 13.7K |  4.6K  | x 0.34 |
|     block.rb      | 13.8K |  4.6K  | x 0.33 |
|    ao_bench.rb    | 14.5K | 21.7K  | x 1.50 |
|     optcarrot     | 68.4K | 90.8K  | x 1.33 |
