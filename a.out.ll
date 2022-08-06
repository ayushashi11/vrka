; ModuleID = "a.out.ll"
target triple = "unknown-unknown-unknown"
target datalayout = ""

define i64 @"main"() 
{
entry0:
  %".2" = sitofp i64 2 to float
  %".3" = fsub float 0x4016000000000000, %".2"
  %".4" = bitcast [4 x i8]* @"fstrf" to i8*
  %".5" = fpext float %".3" to double
  %".6" = call i32 (i8*, ...) @"printf"(i8* %".4", double %".5")
  ret i64 0
}

@"fstri" = internal constant [4 x i8] c"%d\0a\00"
@"fstrf" = internal constant [4 x i8] c"%f\0a\00"
declare i32 @"printf"(i8* %".1", ...) 
