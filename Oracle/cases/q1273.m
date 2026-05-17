// SplitTextByPositions returns a splitter that cuts text at
            // the given 0-based offsets. {0,3,5} → ["abc","de","fgh"].
            Splitter.SplitTextByPositions({0, 3, 5})("abcdefgh")
