# Vrka
Vrka is a general purpose statically typed language Im writing just to understand how llvm and wasm work. Its syntax is partly based on python but I decided to not go  with an indented grammar cause I was having issues with multiline lambdas. There are other grammar quirks I need to fix, as well as order the code in a better way.
## Design philosophy
I was quite inspired by zig overall and rusts type safety. I plan on utilising compile time reference counting instead of a borrow checker. There is no plan as of yet what to do with cyclic references, maybe I would just equate their reference counts and decrement together or put the object with a lower reference under the higher one, essentially creating a reference tree of sorts.
## Feature list
- [x] Basic types
- [x] Variables
- [ ] Stdlib Types
-- [x] Strings
--- [ ] concatenation
--- [ ] format strings
-- [ ] Arrays
-- [ ] Optionals
-- [x] References
--- [ ] Null Checking
--- [ ] Boxed reference type
- [x] Functions
-- [ ] Default/Optional arguments
-- [ ] Named arguments
-- [ ] Closures(Lambdas) (trying to figure out if I should do static capture or dynamic capture and how it would affect reference counting)
-- [ ] Iterators
- [x] Control Flow
-- [x] If elif else
-- [x] While
-- [ ] For (Would do after arrays and iterators)
-- [ ] Match
-- [ ] Break/Continue
-- [ ] Named Loop Blocks
- [x] Classes
-- [x] member access
-- [ ] getters and setters 
-- [ ] functions
-- [ ] generics
- [ ] Traits
- [ ] Enums
-- [ ] Tagged Enums
-- [ ] functions
- [ ] Error Handling
- [ ] Modules
