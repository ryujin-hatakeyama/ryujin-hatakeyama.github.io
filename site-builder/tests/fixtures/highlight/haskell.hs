{-# LANGUAGE GADTs #-}
-- | A typed expression language, evaluated by structural recursion.
module Eval (Expr (..), Name (..), eval, greeting) where

data Expr a where
  Lit :: Int -> Expr Int
  Add :: Expr Int -> Expr Int -> Expr Int
  If  :: Expr Bool -> Expr a -> Expr a -> Expr a

newtype Name = Name String deriving (Eq, Show)

eval :: Expr a -> a
eval (Lit n)    = n
eval (Add x y)  = eval x + eval y
eval (If c t e) = if eval c then t' else e'
  where t' = eval t
        e' = eval e

greeting :: Name -> String
greeting (Name n) = "hello, " <> n ++ "\n"

compose :: (b -> c) -> (a -> b) -> a -> c
compose f g = f . g
